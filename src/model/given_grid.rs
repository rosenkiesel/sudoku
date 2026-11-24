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

    /// Parse a grid string. Separators `,` and `;` are optional (a flat N²
    /// string also parses).
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
                if y > 0 && y % size.bx == 0 {
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
