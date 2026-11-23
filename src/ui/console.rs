use crate::model::{grid, request::SolveRequest, variables, ModelError, Size};
use crate::solver::Assignment;
use crate::ui::render::{self, Givens};
use crate::ui::{Command, Ui, ViewMode};
use std::io::{self, BufRead, Write};

pub const GIVEN_WEIGHT: f64 = 10.0;
pub const DEFAULT_SEED: u64 = 0x5EED_5EED_5EED_5EED;

const HELP: &str = "\
commands:
  new <bx> <by>     start a puzzle of size (Bx,By) — e.g. new 3 3 (9x9) or new 2 3 (6x6)
  rand [seed]       fill the target base with uniform random in [-1,+1]
  set <x> <y> <n>   add a given: target[idx(x,y,n)] += 10.0 on top of the base
  unset <x> <y>     remove the given at cell (x,y)
  load <grid>       load givens from a grid string (rows ';', block columns ',')
  solve [eps]       solve along the central path (default eps 1e-6)
  view winner       argmax per cell + conflicting rows/columns (block borders '#')
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

/// Console frontend: owns the current size, the random target base and the
/// givens; renders the two output views.
pub struct ConsoleUi {
    size: Option<Size>,
    base: Vec<f64>,
    seed: u64,
    givens: Givens,
    assignment: Option<Assignment>,
    view: ViewMode,
}

impl ConsoleUi {
    pub fn new() -> Self {
        Self {
            size: None,
            base: Vec::new(),
            seed: DEFAULT_SEED,
            givens: Givens::new(),
            assignment: None,
            view: ViewMode::Winner,
        }
    }

    fn require_size(&self) -> Result<Size, ModelError> {
        self.size.ok_or(ModelError::NoPuzzle)
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
                    self.size = Some(size);
                    self.base = vec![0.0; size.num_vars()];
                    self.givens.clear();
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
                    self.givens.clear();
                    self.assignment = None;
                }
                Err(e) => self.note(&format!("error: {e}")),
            },
            Command::Set { x, y, n } => match self.require_size() {
                Ok(size) => {
                    let checked = size.check_cell(*x, *y).and_then(|()| size.check_digit(*n));
                    match checked {
                        Ok(()) => {
                            self.givens.insert((*x, *y), *n);
                            self.assignment = None;
                        }
                        Err(e) => self.note(&format!("error: {e}")),
                    }
                }
                Err(e) => self.note(&format!("error: {e}")),
            },
            Command::Unset { x, y } => {
                self.givens.remove(&(*x, *y));
                self.assignment = None;
            }
            Command::Load(text) => match self.require_size() {
                Ok(size) => match grid::parse_grid(&size, text) {
                    Ok(cells) => {
                        self.givens.clear();
                        for (x, y, d) in cells {
                            self.givens.insert((x, y), d);
                        }
                        self.assignment = None;
                    }
                    Err(e) => self.note(&format!("error: {e}")),
                },
                Err(e) => self.note(&format!("error: {e}")),
            },
            Command::Clear => {
                self.givens.clear();
                self.assignment = None;
            }
            Command::View(mode) => self.view = *mode,
            Command::Help => self.note(HELP),
            Command::Show | Command::Solve { .. } | Command::Quit => {}
        }
        Ok(())
    }

    fn solve_request(&self, epsilon: f64) -> Option<SolveRequest> {
        let size = self.size?;
        let mut target = self.base.clone();
        for (&(x, y), &d) in &self.givens {
            target[variables::idx(&size, x, y, d)] += GIVEN_WEIGHT;
        }
        SolveRequest::new(size, target, epsilon).ok()
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
        let Some(size) = self.size else {
            return writeln!(w, "(no puzzle yet — start with: new 3 3)");
        };
        writeln!(
            w,
            "size (Bx,By)=({},{})  N={}  vars={}  givens={}  seed={}",
            size.bx,
            size.by,
            size.n(),
            size.num_vars(),
            self.givens.len(),
            self.seed
        )?;
        match (&self.assignment, self.view) {
            (Some(a), ViewMode::Winner) => {
                render::render_winner(&mut w, &size, &self.givens, &a.values)
            }
            (Some(a), ViewMode::Probabilities) => render::render_probs(&mut w, &size, &a.values),
            _ => render::render_grid(&mut w, &size, |x, y| match self.givens.get(&(x, y)) {
                Some(d) => format!("{d} "),
                None => ". ".to_string(),
            }),
        }?;
        writeln!(
            w,
            "grid: {}",
            grid::format_grid(&size, |x, y| self.givens.get(&(x, y)).copied())
        )
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
        "solve" => Ok(Command::Solve {
            epsilon: match tokens.get(1) {
                None => None,
                Some(s) => Some(
                    s.parse::<f64>()
                        .map_err(|_| format!("invalid epsilon '{s}'"))?,
                ),
            },
        }),
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
