pub mod render;

use crate::model::given_grid::GivenGrid;
use crate::model::request::SolveRequest;
use crate::model::{grid_indices, ModelError, Size};
use crate::solver::Assignment;
use crate::ui::{Command, Ui, ViewMode, DEFAULT_EPSILON};
use std::io::{self, BufRead, Write};

pub const GIVEN_WEIGHT: f64 = 10.0;
pub const DEFAULT_SEED: u64 = 0x5EED_5EED_5EED_5EED;

const HELP: &str = "\
commands:
  new <bx> <by>     start a puzzle of size (Bx,By) — e.g. new 3 3 (9x9) or new 2 3 (6x6)
  rand [seed]       fill the target base with uniform random in [-1,+1] (givens are kept)
  set <x> <y> <n>   add a given: target[idx(x,y,n)] += 10.0 on top of the base
  unset <x> <y>     remove the given at cell (x,y)
  load <grid>       load givens from a grid string (rows ';', block columns ',');
                    without a current puzzle the size (Bx,By) is inferred from the string
  solve             solve along the central path down to the current epsilon
  eps+ | eps-       multiply / divide the current epsilon by 10
  view winner       argmax per cell; givens '*', conflicting entries 'c' (block borders '#')
  view probs        per-cell percentage mini-grid (cells ':' within a block, block borders '#')
  show              re-render
  clear             remove all givens
  help              this text
  quit              exit
grid string example (9x9): 530,070,000;600,195,000;098,000,060;...";

/// SplitMix64 — small deterministic RNG so the project stays dependency-free.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1_u64 << 53) as f64
    }
}

/// Console frontend: owns the given grid, the random target base and the
/// current epsilon; renders the two output views. The last assignment is
/// reused as the start state of the next solve (warm start).
pub struct ConsoleUi {
    grid: Option<GivenGrid>,
    base: Vec<f64>,
    seed: u64,
    epsilon: f64,
    assignment: Option<Assignment>,
    view: ViewMode,
}

impl ConsoleUi {
    pub fn new() -> Self {
        Self {
            grid: None,
            base: Vec::new(),
            seed: DEFAULT_SEED,
            epsilon: DEFAULT_EPSILON,
            assignment: None,
            view: ViewMode::Winner,
        }
    }

    fn require_size(&self) -> Result<Size, ModelError> {
        self.grid
            .as_ref()
            .map(|g| g.size())
            .ok_or(ModelError::NoPuzzle)
    }
}

impl Default for ConsoleUi {
    fn default() -> Self {
        Self::new()
    }
}

impl Ui for ConsoleUi {
    fn next_command(&mut self) -> io::Result<Command> {
        let stdin = io::stdin();
        loop {
            print!("> ");
            io::stdout().flush()?;
            let mut line = String::new();
            if stdin.lock().read_line(&mut line)? == 0 {
                return Ok(Command::Quit);
            }
            match parse_command(line.trim()) {
                Ok(command) => return Ok(command),
                Err(message) => self.note(&format!("error: {message}")),
            }
        }
    }

    fn apply(&mut self, command: &Command) -> io::Result<()> {
        match command {
            Command::New { bx, by } => match Size::new(*bx, *by) {
                Ok(size) => {
                    self.grid = Some(GivenGrid::new(size));
                    self.base = vec![0.0; size.num_vars()];
                    self.assignment = None;
                    self.seed = DEFAULT_SEED;
                }
                Err(e) => self.note(&format!("error: {e}")),
            },
            Command::Rand { seed } => match self.require_size() {
                Ok(size) => {
                    self.seed = seed.unwrap_or(DEFAULT_SEED);
                    let mut rng = Rng(self.seed);
                    self.base = (0..size.num_vars())
                        .map(|_| rng.next_f64() * 2.0 - 1.0)
                        .collect();
                    self.assignment = None;
                }
                Err(e) => self.note(&format!("error: {e}")),
            },
            Command::Set { x, y, n } => match &mut self.grid {
                Some(grid) => {
                    if let Err(e) = grid.set(*x, *y, *n) {
                        self.note(&format!("error: {e}"));
                    } else {
                        self.assignment = None;
                    }
                }
                None => self.note(&format!("error: {}", ModelError::NoPuzzle)),
            },
            Command::Unset { x, y } => {
                if let Some(grid) = &mut self.grid {
                    grid.unset(*x, *y);
                    self.assignment = None;
                }
            }
            Command::Load(text) => match &self.grid {
                // puzzle exists: parse with its size
                Some(grid) => {
                    let size = grid.size();
                    match GivenGrid::parse(&size, text) {
                        Ok(grid) => {
                            self.grid = Some(grid);
                            self.assignment = None;
                        }
                        Err(e) => self.note(&format!("error: {e}")),
                    }
                }
                // no puzzle yet: infer the size from the string's separators
                None => match GivenGrid::parse_auto(text) {
                    Ok(grid) => {
                        let size = grid.size();
                        self.base = vec![0.0; size.num_vars()];
                        self.grid = Some(grid);
                        self.assignment = None;
                    }
                    Err(e) => self.note(&format!("error: {e}")),
                },
            },
            Command::Clear => {
                if let Some(grid) = &mut self.grid {
                    grid.clear();
                    self.assignment = None;
                }
            }
            Command::EpsPlus => {
                self.epsilon = (self.epsilon * 10.0).min(1.0);
                self.note(&format!("epsilon = {:.0e}", self.epsilon));
            }
            Command::EpsMinus => {
                self.epsilon = (self.epsilon / 10.0).max(1e-12);
                self.note(&format!("epsilon = {:.0e}", self.epsilon));
            }
            Command::View(mode) => self.view = *mode,
            Command::Help => self.note(HELP),
            Command::Show | Command::Solve | Command::Quit => {}
        }
        Ok(())
    }

    fn solve_request(&self) -> Option<SolveRequest> {
        let grid = self.grid.as_ref()?;
        let size = grid.size();
        let mut target = self.base.clone();
        for (&(x, y), &d) in grid.givens() {
            target[grid_indices::idx(&size, x, y, d)] += GIVEN_WEIGHT;
        }
        // warm start: reuse the last assignment if it matches the size,
        // otherwise the default start (1/N in every variable)
        let start = match &self.assignment {
            Some(a) if a.values.len() == size.num_vars() => a.values.clone(),
            _ => crate::model::request::default_start(&size),
        };
        SolveRequest::with_start(size, target, self.epsilon, start).ok()
    }

    fn set_assignment(&mut self, assignment: Option<Assignment>) {
        self.assignment = assignment;
    }

    fn note(&mut self, message: &str) {
        println!("{message}");
    }

    fn render(&mut self) -> io::Result<()> {
        let stdout = io::stdout();
        let mut w = stdout.lock();
        let Some(grid) = &self.grid else {
            return writeln!(w, "(no puzzle yet — start with: new 3 3)");
        };
        let size = grid.size();
        writeln!(
            w,
            "size (Bx,By)=({},{})  N={}  vars={}  givens={}  seed={}  eps={:.0e}",
            size.bx,
            size.by,
            size.n(),
            size.num_vars(),
            grid.givens().len(),
            self.seed,
            self.epsilon
        )?;
        match (&self.assignment, self.view) {
            (Some(a), ViewMode::Winner) => render::render_winner(&mut w, grid, &a.values),
            (Some(a), ViewMode::Probabilities) => render::render_probs(&mut w, &size, &a.values),
            _ => render::render_grid(&mut w, &size, 2, |x, y| match grid.get(x, y) {
                Some(d) => format!("{d} "),
                None => ". ".to_string(),
            }),
        }?;
        writeln!(w, "grid: {grid}")
    }
}

fn parse_command(line: &str) -> Result<Command, String> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let Some(head) = tokens.first() else {
        return Err("empty command".to_string());
    };
    match *head {
        "quit" | "exit" => Ok(Command::Quit),
        "new" => {
            let bx = parse_usize(tokens.get(1), "bx")?;
            let by = parse_usize(tokens.get(2), "by")?;
            Ok(Command::New { bx, by })
        }
        "rand" => Ok(Command::Rand {
            seed: match tokens.get(1) {
                None => None,
                Some(s) => Some(
                    s.parse::<u64>()
                        .map_err(|_| format!("invalid seed '{s}'"))?,
                ),
            },
        }),
        "set" => {
            let x = parse_usize(tokens.get(1), "x")?;
            let y = parse_usize(tokens.get(2), "y")?;
            let n = parse_u8(tokens.get(3), "n")?;
            Ok(Command::Set { x, y, n })
        }
        "unset" => {
            let x = parse_usize(tokens.get(1), "x")?;
            let y = parse_usize(tokens.get(2), "y")?;
            Ok(Command::Unset { x, y })
        }
        "solve" => Ok(Command::Solve),
        "eps+" => Ok(Command::EpsPlus),
        "eps-" => Ok(Command::EpsMinus),
        "view" => match tokens.get(1) {
            Some(&"winner") => Ok(Command::View(ViewMode::Winner)),
            Some(&"probs") => Ok(Command::View(ViewMode::Probabilities)),
            _ => Err("usage: view winner|probs".to_string()),
        },
        "show" => Ok(Command::Show),
        "help" | "?" => Ok(Command::Help),
        "load" => match tokens.get(1) {
            Some(s) => Ok(Command::Load((*s).to_string())),
            None => Err("usage: load <string>".to_string()),
        },
        "clear" => Ok(Command::Clear),
        other => Err(format!("unknown command '{other}'")),
    }
}

fn parse_usize(value: Option<&&str>, name: &str) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("missing {name}"))?
        .parse::<usize>()
        .map_err(|_| format!("invalid {name}"))
}

fn parse_u8(value: Option<&&str>, name: &str) -> Result<u8, String> {
    value
        .ok_or_else(|| format!("missing {name}"))?
        .parse::<u8>()
        .map_err(|_| format!("invalid {name}"))
}
