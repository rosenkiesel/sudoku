pub mod constraints;
pub mod request;
pub mod variables;

use std::fmt;

/// Puzzle size given as block dimensions: a `N x N` grid with `N = bx * by`,
/// blocks of `bx` columns x `by` rows arranged in a `by x bx` pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Size {
    pub bx: usize,
    pub by: usize,
}

impl Size {
    pub fn new(bx: usize, by: usize) -> Result<Self, ModelError> {
        if bx == 0 || by == 0 || bx * by > 64 {
            return Err(ModelError::InvalidSize { bx, by });
        }
        Ok(Self { bx, by })
    }

    /// Grid edge length N; digits are 1..=N.
    pub fn n(&self) -> usize {
        self.bx * self.by
    }

    /// Number of probability variables p(x, y, n): N^3.
    pub fn num_vars(&self) -> usize {
        let n = self.n();
        n * n * n
    }

    /// Number of block columns.
    pub fn block_columns(&self) -> usize {
        self.by
    }

    /// Number of block rows.
    pub fn block_rows(&self) -> usize {
        self.bx
    }

    pub fn check_cell(&self, x: usize, y: usize) -> Result<(), ModelError> {
        let n = self.n();
        if x >= n || y >= n {
            Err(ModelError::CellOutOfRange { x, y, n })
        } else {
            Ok(())
        }
    }

    pub fn check_digit(&self, n: u8) -> Result<(), ModelError> {
        if n == 0 || n as usize > self.n() {
            Err(ModelError::DigitOutOfRange { n, max: self.n() })
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub x: usize,
    pub y: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModelError {
    InvalidSize { bx: usize, by: usize },
    CellOutOfRange { x: usize, y: usize, n: usize },
    DigitOutOfRange { n: u8, max: usize },
    TargetLength { expected: usize, actual: usize },
    InvalidEpsilon { epsilon: f64 },
    NoPuzzle,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize { bx, by } => {
                write!(
                    f,
                    "invalid size ({bx},{by}): need bx,by >= 1 and bx*by <= 64"
                )
            }
            Self::CellOutOfRange { x, y, n } => {
                write!(f, "cell ({x},{y}) out of range for N={n}")
            }
            Self::DigitOutOfRange { n, max } => {
                write!(f, "digit {n} out of range 1..={max}")
            }
            Self::TargetLength { expected, actual } => {
                write!(f, "target vector has length {actual}, expected {expected}")
            }
            Self::InvalidEpsilon { epsilon } => {
                write!(f, "epsilon must be a positive finite number, got {epsilon}")
            }
            Self::NoPuzzle => write!(f, "no puzzle yet — start with: new <bx> <by>"),
        }
    }
}

impl std::error::Error for ModelError {}
