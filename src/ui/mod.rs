pub mod console;

use crate::model::request::SolveRequest;
use crate::solver::Assignment;
use std::fmt;
use std::io;

pub const DEFAULT_EPSILON: f64 = 1e-6;

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

/// Anything user-facing goes through this trait; `app` is generic over it.
pub trait Ui {
    fn next_command(&mut self) -> io::Result<Command>;
    fn apply(&mut self, command: &Command) -> io::Result<()>;
    fn solve_request(&self) -> Option<SolveRequest>;
    fn set_assignment(&mut self, assignment: Option<Assignment>);
    fn note(&mut self, message: &str);
    fn render(&mut self) -> io::Result<()>;
}
