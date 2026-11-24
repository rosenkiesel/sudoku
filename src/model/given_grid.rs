use crate::model::{ModelError, Size};
use std::collections::BTreeMap;
use std::fmt;

/// The givens of a puzzle on a grid of size `(bx, by)`.
///
/// String representation: rows separated by `;`, block columns within a row
/// by `,`. Digits are `1..=9`, then hex `a..`; `.` or `0` means empty.
/// Example 9×9: `530,070,000;600,195,000;098,000,060;…`
/// Example 6×6 with (Bx,By)=(2,3): `1.,3.,5.;.4,.6,.2;5.,.2,.4;…`
#[derive(Clone, Debug)]
pub struct GivenGrid {
    size: Size,
    givens: BTreeMap<(usize, usize), u8>,
}

impl GivenGrid {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            givens: BTreeMap::new(),
        }
    }

    /// Dimensions of the game: `(Bx, By)`.
    pub fn dimensions(&self) -> (usize, usize) {
        (self.size.bx, self.size.by)
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn givens(&self) -> &BTreeMap<(usize, usize), u8> {
        &self.givens
    }

    pub fn get(&self, x: usize, y: usize) -> Option<u8> {
        self.givens.get(&(x, y)).copied()
    }

    pub fn set(&mut self, x: usize, y: usize, n: u8) -> Result<(), ModelError> {
        self.size.check_cell(x, y)?;
        self.size.check_digit(n)?;
        self.givens.insert((x, y), n);
        Ok(())
    }

    pub fn unset(&mut self, x: usize, y: usize) {
        self.givens.remove(&(x, y));
    }

    pub fn clear(&mut self) {
        self.givens.clear();
    }

    /// Parse a grid string and infer the size `(Bx, By)` from the separators:
    /// `;`-separated rows give N, the `,`-separated block columns in a row give
    /// `By` (segment count), and the segment length gives `Bx` (N = Bx·By).
    /// A flat N² string without separators is read as a square grid
    /// (Bx = By = √N) when N is a perfect square.
    pub fn parse_auto(text: &str) -> Result<Self, ModelError> {
        if text.contains(';') || text.contains(',') {
            let rows: Vec<&str> = text.split(';').collect();
            let n = rows.len();
            let first = rows[0].split(',').collect::<Vec<_>>();
            let by = first.len();
            let bx = first[0].chars().count();
            for row in &rows {
                let segs: Vec<&str> = row.split(',').collect();
                if segs.len() != by || segs.iter().any(|s| s.chars().count() != bx) {
                    return Err(ModelError::InvalidGrid(format!(
                        "inconsistent separators: row '{row}' does not match {by} block columns of width {bx}"
                    )));
                }
            }
            if bx == 0 || by == 0 || bx * by != n {
                return Err(ModelError::InvalidGrid(format!(
                    "inconsistent grid: {n} rows do not match Bx={bx} x By={by}"
                )));
            }
            let size = Size::new(bx, by)?;
            Self::parse(&size, text)
        } else {
            // flat N^2 string: grid edge N = sqrt(len); assume square blocks,
            // i.e. Bx = By = sqrt(N), when that is an integer
            let len = text.chars().count();
            let n = (len as f64).sqrt().round() as usize;
            let b = (n as f64).sqrt().round() as usize;
            if n * n != len || b * b != n {
                return Err(ModelError::InvalidGrid(format!(
                    "cannot infer size from a flat string of length {len}; use new <bx> <by> first or add ';' / ',' separators"
                )));
            }
            let size = Size::new(b, b)?;
            Self::parse(&size, text)
        }
    }

    /// Parse a grid string for a known size. Separators `,` and `;` are
    /// optional (a flat N² string also parses).
    pub fn parse(size: &Size, text: &str) -> Result<Self, ModelError> {
        let n = size.n();
        let chars: Vec<char> = text.chars().filter(|c| *c != ',' && *c != ';').collect();
        if chars.len() != n * n {
            return Err(ModelError::InvalidGrid(format!(
                "expected {} cells, got {}",
                n * n,
                chars.len()
            )));
        }
        let mut grid = Self::new(*size);
        for (i, ch) in chars.iter().enumerate() {
            if *ch == '.' || *ch == '0' {
                continue;
            }
            match ch.to_digit(36) {
                Some(d) if d >= 1 && (d as usize) <= n => {
                    grid.givens.insert((i / n, i % n), d as u8);
                }
                _ => return Err(ModelError::InvalidGrid(format!("invalid digit '{ch}'"))),
            }
        }
        Ok(grid)
    }

    /// String form: rows `;`, block columns `,`.
    pub fn format(&self) -> String {
        let size = self.size;
        let n = size.n();
        let mut rows = Vec::with_capacity(n);
        for x in 0..n {
            let mut row = String::with_capacity(n + size.block_columns());
            for y in 0..n {
                if y > 0 && y.is_multiple_of(size.bx) {
                    row.push(',');
                }
                row.push(match self.get(x, y) {
                    Some(d) => char::from_digit(d as u32, 36).unwrap(),
                    None => '.',
                });
            }
            rows.push(row);
        }
        rows.join(";")
    }
}

impl fmt::Display for GivenGrid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format())
    }
}
