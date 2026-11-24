use crate::model::given_grid::GivenGrid;
use crate::model::is_multiple_of;
use crate::model::{grid_indices, Size};
use crate::ui::{conflict_cells, conflicts, winners};
use std::io::{self, Write};

/// Console-display-only scaling: probabilities are shown as `99 * p` percent,
/// so every mini-cell is at most `99%` and `100%` never occurs. The
/// model/solver values stay raw 0..1 — other UIs (e.g. a future HTML
/// frontend) decide themselves how to render 0..100% (e.g. color-coded).
pub const DISPLAY_SCALE: f64 = 0.99;

/// Grid view with `#` as block delimiter: vertical `#` between block columns,
/// a `#` line between block rows, plain space between ordinary cells.
/// `cell_width` is the fixed width of one cell's text.
pub fn render_grid(
    w: &mut impl Write,
    size: &Size,
    cell_width: usize,
    cell_text: impl Fn(usize, usize) -> String,
) -> io::Result<()> {
    let n = size.n();
    let width = cell_width * n + (n - 1);
    for x in 0..n {
        if x > 0 && is_multiple_of(x, size.by) {
            writeln!(w, "{}", "#".repeat(width))?;
        }
        let mut line = String::new();
        for y in 0..n {
            if y > 0 {
                line.push(if is_multiple_of(y, size.bx) { '#' } else { ' ' });
            }
            line.push_str(&cell_text(x, y));
        }
        writeln!(w, "{line}")?;
    }
    Ok(())
}

/// Winner view: argmax digit per cell. Givens get a `*` suffix, conflicting
/// entries (winner digit duplicated in their row or column) a `c` suffix —
/// both if both apply. Afterwards the conflicting rows/columns are listed.
pub fn render_winner(w: &mut impl Write, grid: &GivenGrid, values: &[f64]) -> io::Result<()> {
    let size = grid.size();
    let n = size.n();
    let wins = winners(&size, values);
    let flags = conflict_cells(&size, &wins);
    render_grid(w, &size, 3, |x, y| {
        let mut s = format!("{}", wins[x * n + y]);
        if grid.get(x, y).is_some() {
            s.push('*');
        }
        if flags[x * n + y] {
            s.push('c');
        }
        while s.len() < 3 {
            s.push(' ');
        }
        s
    })?;
    let (rows, cols) = conflicts(&size, &wins);
    writeln!(w, "conflicting rows: {rows:?}")?;
    writeln!(w, "conflicting columns: {cols:?}")?;
    Ok(())
}

/// Probability view: per cell a `by × bx` mini-grid with the percentage of
/// each digit (mini-cell (i, j) shows digit i*bx + j + 1). Within a block,
/// cells are delimited by `:` and cell rows by `-` lines; block boundaries
/// stay `#` (vertical `#` between block columns, `#` line between block rows).
/// Mini-cells are zero-padded (`00%`..`99%`, console scales by DISPLAY_SCALE).
pub fn render_probs(w: &mut impl Write, size: &Size, values: &[f64]) -> io::Result<()> {
    let n = size.n();
    let mut width = 0_usize;
    for x in 0..n {
        for i in 0..size.by {
            let mut groups = Vec::with_capacity(size.block_columns());
            for cx in 0..size.block_columns() {
                let mut cells = Vec::with_capacity(size.bx);
                for y in cx * size.bx..(cx + 1) * size.bx {
                    let mut cell = String::new();
                    for j in 0..size.bx {
                        let d = (i * size.bx + j + 1) as u8;
                        let p = values[grid_indices::idx(size, x, y, d)];
                        if j > 0 {
                            cell.push(' ');
                        }
                        // zero-padded "<nn>%": mini-cells are separated by
                        // exactly "% ", block boundaries read "%#", and every
                        // mini-cell is 3 chars wide
                        cell.push_str(&format!("{:02.0}%", 100.0 * p * DISPLAY_SCALE));
                    }
                    cells.push(cell);
                }
                groups.push(cells.join(":"));
            }
            let line = groups.join("#");
            width = width.max(line.len());
            writeln!(w, "{line}")?;
        }
        if x + 1 < n {
            let sep = if is_multiple_of(x + 1, size.by) {
                '#'
            } else {
                '-'
            };
            writeln!(w, "{}", sep.to_string().repeat(width))?;
        }
    }
    Ok(())
}
