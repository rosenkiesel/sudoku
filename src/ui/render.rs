use crate::model::{variables, Size};
use std::collections::BTreeMap;
use std::io::{self, Write};

pub type Givens = BTreeMap<(usize, usize), u8>;

/// Winning digit of a cell: argmax over n of p(x, y, n).
pub fn winner(size: &Size, values: &[f64], x: usize, y: usize) -> u8 {
    let n = size.n();
    let mut best = 1_u8;
    let mut best_p = f64::NEG_INFINITY;
    for d in 1..=n {
        let p = values[variables::idx(size, x, y, d as u8)];
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

/// Rows/columns in which some digit occurs more than once among the winners.
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

/// Grid view with `#` as block delimiter: vertical `#` between block columns,
/// a `#` line between block rows, plain space between ordinary cells.
pub fn render_grid(
    w: &mut impl Write,
    size: &Size,
    cell_text: impl Fn(usize, usize) -> String,
) -> io::Result<()> {
    let n = size.n();
    let width = 2 * n + size.block_columns() - 1;
    for x in 0..n {
        if x > 0 && x % size.by == 0 {
            writeln!(w, "{}", "#".repeat(width))?;
        }
        let mut line = String::new();
        for y in 0..n {
            if y > 0 {
                line.push(if y % size.bx == 0 { '#' } else { ' ' });
            }
            line.push_str(&cell_text(x, y));
        }
        writeln!(w, "{line}")?;
    }
    Ok(())
}

/// Winner view: argmax digit per cell (givens marked with `*`), then the
/// rows/columns that contain conflicts are flagged.
pub fn render_winner(
    w: &mut impl Write,
    size: &Size,
    givens: &Givens,
    values: &[f64],
) -> io::Result<()> {
    let n = size.n();
    let wins = winners(size, values);
    render_grid(w, size, |x, y| {
        let d = wins[x * n + y];
        if givens.contains_key(&(x, y)) {
            format!("{d}*")
        } else {
            format!("{d} ")
        }
    })?;
    let (rows, cols) = conflicts(size, &wins);
    writeln!(w, "conflicting rows: {rows:?}")?;
    writeln!(w, "conflicting columns: {cols:?}")?;
    Ok(())
}

/// Probability view: per cell a `by × bx` mini-grid with the percentage of
/// each digit (mini-cell (i, j) shows digit i*bx + j + 1). Cells are
/// delimited by `:` within a row and by `-` lines between cell rows.
pub fn render_probs(w: &mut impl Write, size: &Size, values: &[f64]) -> io::Result<()> {
    let n = size.n();
    let mut width = 0_usize;
    for x in 0..n {
        for i in 0..size.by {
            let mut cells = Vec::with_capacity(n);
            for y in 0..n {
                let mut cell = String::new();
                for j in 0..size.bx {
                    let d = (i * size.bx + j + 1) as u8;
                    let p = values[variables::idx(size, x, y, d)];
                    if j > 0 {
                        cell.push(' ');
                    }
                    cell.push_str(&format!("{:>3.0}%", 100.0 * p));
                }
                cells.push(cell);
            }
            let line = cells.join(":");
            width = width.max(line.len());
            writeln!(w, "{line}")?;
        }
        if x + 1 < n {
            writeln!(w, "{}", "-".repeat(width))?;
        }
    }
    Ok(())
}
