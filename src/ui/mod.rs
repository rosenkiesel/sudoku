pub mod console;

use crate::model::request::SolveRequest;
use crate::solver::Assignment;
use std::collections::BTreeMap;
use std::io;

pub const DEFAULT_EPSILON: f64 = 1e-6;

/// Generic helper: givens of a puzzle as (x, y) -> digit.
pub type Givens = BTreeMap<(usize, usize), u8>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Winner,
    Probabilities,
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
