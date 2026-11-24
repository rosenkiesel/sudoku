use crate::model::{grid_indices, Size};

pub type SparseRow = Vec<(usize, f64)>;

/// Rules as linear equations `A * p = b` plus per-variable bounds.
/// Contains no givens — those enter only through the target vector.
#[derive(Clone, Debug)]
pub struct LinearSystem {
    pub size: Size,
    pub num_vars: usize,
    pub rows: Vec<(SparseRow, f64)>,
    pub lower: Vec<f64>,
    pub upper: Vec<f64>,
}

impl LinearSystem {
    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }
}

/// Block coordinates (cx, cy) of cell (x, y): cx in 0..by, cy in 0..bx.
pub fn block_of(size: &Size, x: usize, y: usize) -> (usize, usize) {
    (y / size.bx, x / size.by)
}

/// Build the rule system: cell sums, row/column/block rules (4*N^2 rows),
/// bounds 0 <= p <= 1.
pub fn build_system(size: &Size) -> LinearSystem {
    let n = size.n();
    let num_vars = size.num_vars();
    let mut rows: Vec<(SparseRow, f64)> = Vec::with_capacity(4 * n * n);

    // cell sum: every cell carries exactly one digit
    for x in 0..n {
        for y in 0..n {
            let row: SparseRow = (1..=n)
                .map(|d| (grid_indices::idx(size, x, y, d as u8), 1.0))
                .collect();
            rows.push((row, 1.0));
        }
    }
    // row rule: each digit exactly once per row
    for x in 0..n {
        for d in 1..=n {
            let row: SparseRow = (0..n)
                .map(|y| (grid_indices::idx(size, x, y, d as u8), 1.0))
                .collect();
            rows.push((row, 1.0));
        }
    }
    // column rule: each digit exactly once per column
    for y in 0..n {
        for d in 1..=n {
            let row: SparseRow = (0..n)
                .map(|x| (grid_indices::idx(size, x, y, d as u8), 1.0))
                .collect();
            rows.push((row, 1.0));
        }
    }
    // block rule: each digit exactly once per block
    for cy in 0..size.block_rows() {
        for cx in 0..size.block_columns() {
            for d in 1..=n {
                let mut row: SparseRow = Vec::with_capacity(n);
                for x in cy * size.by..(cy + 1) * size.by {
                    for y in cx * size.bx..(cx + 1) * size.bx {
                        row.push((grid_indices::idx(size, x, y, d as u8), 1.0));
                    }
                }
                rows.push((row, 1.0));
            }
        }
    }

    LinearSystem {
        size: *size,
        num_vars,
        rows,
        lower: vec![0.0; num_vars],
        upper: vec![1.0; num_vars],
    }
}
