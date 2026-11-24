pub mod central_path;

use crate::model::game_rules::LinearSystem;
use crate::model::request::SolveRequest;
use std::fmt;

/// A solved assignment: one value per variable p(x, y, n).
#[derive(Clone, Debug)]
pub struct Assignment {
    pub values: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SolverError {
    Infeasible,
    NoIntegralSolution,
    Backend(String),
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Infeasible => write!(f, "infeasible"),
            Self::NoIntegralSolution => write!(f, "no integral solution found"),
            Self::Backend(message) => write!(f, "backend error: {message}"),
        }
    }
}

impl std::error::Error for SolverError {}

pub trait Solver {
    fn solve(
        &self,
        system: &LinearSystem,
        request: &SolveRequest,
    ) -> Result<Assignment, SolverError>;
}
