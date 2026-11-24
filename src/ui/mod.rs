pub mod console;
#[cfg(feature = "web")]
pub mod web;

use crate::model::given_grid::GivenGrid;
use crate::model::request::{default_start, SolveRequest};
use crate::model::{grid_indices, ModelError, Size};
use crate::solver::Assignment;
use std::fmt;
use std::io;

pub const DEFAULT_EPSILON: f64 = 1e-6;
pub const GIVEN_WEIGHT: f64 = 10.0;
pub const DEFAULT_SEED: u64 = 0x5EED_5EED_5EED_5EED;
/// Largest grid edge N the console allows (`Size` cap is bx*by <= 64).
pub const MAX_N_CONSOLE: usize = 64;
/// Largest grid edge N the web UI allows (cookie size limit, REQUIREMENTS §6: up to 12×12).
pub const MAX_N_WEB: usize = 12;

pub const HELP: &str = "\
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Winner,
    Probabilities,
}

impl ViewMode {
    /// Short string form (stable identifier for stored state, e.g. cookies).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Winner => "winner",
            Self::Probabilities => "probs",
        }
    }

    /// Parse a view-mode string. Unknown values return None, so callers can
    /// fall back to the default — old cookies with a since-removed mode must
    /// not break.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "winner" => Some(Self::Winner),
            "probs" => Some(Self::Probabilities),
            _ => None,
        }
    }
}

impl fmt::Display for ViewMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    New { bx: usize, by: usize },
    Rand { seed: Option<u64> },
    Set { x: usize, y: usize, n: u8 },
    Unset { x: usize, y: usize },
    Show,
    Help,
    Solve,
    EpsPlus,
    EpsMinus,
    View(ViewMode),
    Load(String),
    Clear,
    Quit,
}

/// SplitMix64 — small deterministic RNG so the core stays dependency-free.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1_u64 << 53) as f64
    }
}

/// Shared UI state (generic helper for every frontend): given grid, random
/// target base, epsilon, last assignment (warm start), view mode.
pub struct UiState {
    pub grid: Option<GivenGrid>,
    pub base: Vec<f64>,
    pub seed: u64,
    pub epsilon: f64,
    pub assignment: Option<Assignment>,
    pub view: ViewMode,
}

impl UiState {
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

    pub fn size(&self) -> Option<Size> {
        self.grid.as_ref().map(|g| g.size())
    }

    /// Apply a non-solve command. `max_n` caps the grid edge N (console: 64,
    /// web: 12). Returns an optional user-facing message (e.g. the new epsilon).
    pub fn apply(&mut self, command: &Command, max_n: usize) -> Result<Option<String>, ModelError> {
        match command {
            Command::New { bx, by } => {
                let size = Size::new(*bx, *by)?;
                if size.n() > max_n {
                    return Err(ModelError::InvalidSize { bx: *bx, by: *by });
                }
                self.grid = Some(GivenGrid::new(size));
                self.base = vec![0.0; size.num_vars()];
                self.assignment = None;
                self.seed = DEFAULT_SEED;
                Ok(None)
            }
            Command::Rand { seed } => {
                let size = self.size().ok_or(ModelError::NoPuzzle)?;
                self.seed = seed.unwrap_or(DEFAULT_SEED);
                let mut rng = Rng::new(self.seed);
                self.base = (0..size.num_vars())
                    .map(|_| rng.next_f64() * 2.0 - 1.0)
                    .collect();
                self.assignment = None;
                Ok(None)
            }
            Command::Set { x, y, n } => {
                let grid = self.grid.as_mut().ok_or(ModelError::NoPuzzle)?;
                grid.set(*x, *y, *n)?;
                self.assignment = None;
                Ok(None)
            }
            Command::Unset { x, y } => {
                if let Some(grid) = &mut self.grid {
                    grid.unset(*x, *y);
                    self.assignment = None;
                }
                Ok(None)
            }
            Command::Load(text) => {
                match &self.grid {
                    Some(grid) => {
                        let size = grid.size();
                        let g = GivenGrid::parse(&size, text)?;
                        self.grid = Some(g);
                    }
                    None => {
                        let g = GivenGrid::parse_auto(text)?;
                        if g.size().n() > max_n {
                            return Err(ModelError::InvalidSize {
                                bx: g.size().bx,
                                by: g.size().by,
                            });
                        }
                        self.base = vec![0.0; g.size().num_vars()];
                        self.grid = Some(g);
                    }
                }
                self.assignment = None;
                Ok(None)
            }
            Command::Clear => {
                if let Some(grid) = &mut self.grid {
                    grid.clear();
                    self.assignment = None;
                }
                Ok(None)
            }
            Command::EpsPlus => {
                self.epsilon = (self.epsilon * 10.0).min(1.0);
                Ok(Some(format!("epsilon = {:.0e}", self.epsilon)))
            }
            Command::EpsMinus => {
                self.epsilon = (self.epsilon / 10.0).max(1e-12);
                Ok(Some(format!("epsilon = {:.0e}", self.epsilon)))
            }
            Command::View(mode) => {
                self.view = *mode;
                Ok(None)
            }
            Command::Show | Command::Help | Command::Solve | Command::Quit => Ok(None),
        }
    }

    pub fn solve_request(&self) -> Option<SolveRequest> {
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
            _ => default_start(&size),
        };
        SolveRequest::with_start(size, target, self.epsilon, start).ok()
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Winning digit of a cell: argmax over n of p(x, y, n).
pub fn winner(size: &Size, values: &[f64], x: usize, y: usize) -> u8 {
    let n = size.n();
    let mut best = 1_u8;
    let mut best_p = f64::NEG_INFINITY;
    for d in 1..=n {
        let p = values[grid_indices::idx(size, x, y, d as u8)];
        if p > best_p {
            best_p = p;
            best = d as u8;
        }
    }
    best
}

/// Winners of all cells, row-major.
pub fn winners(size: &Size, values: &[f64]) -> Vec<u8> {
    let n = size.n();
    let mut out = Vec::with_capacity(n * n);
    for x in 0..n {
        for y in 0..n {
            out.push(winner(size, values, x, y));
        }
    }
    out
}

/// Per-cell conflict flags, row-major: a cell is conflicting if its winner
/// digit occurs more than once in its row or in its column.
pub fn conflict_cells(size: &Size, wins: &[u8]) -> Vec<bool> {
    let n = size.n();
    let mut flags = vec![false; n * n];
    for x in 0..n {
        let mut count = vec![0_usize; n + 1];
        for y in 0..n {
            count[wins[x * n + y] as usize] += 1;
        }
        for y in 0..n {
            if count[wins[x * n + y] as usize] > 1 {
                flags[x * n + y] = true;
            }
        }
    }
    for y in 0..n {
        let mut count = vec![0_usize; n + 1];
        for x in 0..n {
            count[wins[x * n + y] as usize] += 1;
        }
        for x in 0..n {
            if count[wins[x * n + y] as usize] > 1 {
                flags[x * n + y] = true;
            }
        }
    }
    flags
}

/// Rows/columns in which some digit occurs more than once among the winners
/// (i.e. the row/column itself contains a conflict).
pub fn conflicts(size: &Size, wins: &[u8]) -> (Vec<usize>, Vec<usize>) {
    let n = size.n();
    let mut bad_rows = Vec::new();
    for x in 0..n {
        let mut count = vec![0_usize; n + 1];
        for y in 0..n {
            count[wins[x * n + y] as usize] += 1;
        }
        if (1..=n).any(|d| count[d] > 1) {
            bad_rows.push(x);
        }
    }
    let mut bad_cols = Vec::new();
    for y in 0..n {
        let mut count = vec![0_usize; n + 1];
        for x in 0..n {
            count[wins[x * n + y] as usize] += 1;
        }
        if (1..=n).any(|d| count[d] > 1) {
            bad_cols.push(y);
        }
    }
    (bad_rows, bad_cols)
}

pub fn parse_command(line: &str) -> Result<Command, String> {
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
        "view" => match tokens.get(1).and_then(|s| ViewMode::parse(s)) {
            Some(mode) => Ok(Command::View(mode)),
            None => Err("usage: view winner|probs".to_string()),
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

/// A frontend: owns a `UiState`, reads commands, renders to a string
/// (console text or HTML), shows notes.
pub trait Ui {
    fn state(&mut self) -> &mut UiState;
    fn next_command(&mut self) -> io::Result<Command>;
    fn note(&mut self, message: &str);
    fn help_text(&self) -> &'static str;
    fn render(&mut self) -> io::Result<String>;
}
