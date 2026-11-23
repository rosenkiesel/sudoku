pub mod console;
pub mod render;

use crate::model::request::SolveRequest;
use crate::solver::Assignment;
use std::io;

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
    Solve { epsilon: Option<f64> },
    View(ViewMode),
    Load(String),
    Clear,
    Quit,
}

/// Anything user-facing goes through this trait; `app` is generic over it.
pub trait Ui {
    fn next_command(&mut self) -> io::Result<Command>;
    fn apply(&mut self, command: &Command) -> io::Result<()>;
    fn solve_request(&self, epsilon: f64) -> Option<SolveRequest>;
    fn set_assignment(&mut self, assignment: Option<Assignment>);
    fn note(&mut self, message: &str);
    fn render(&mut self) -> io::Result<()>;
}
