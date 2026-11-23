use crate::model::{ModelError, Size};

/// String representation of a grid: rows separated by `;`, block columns
/// within a row separated by `,`. Digits are `1..=9` then hex `a..`; `.` or
/// `0` means empty. Example 9×9: `530,070,000;600,195,000;098,000,060;…`
/// Example 6×6 with (Bx,By)=(2,3): `1.,3.,5.;.4,.6.,2;5.,.2.,4.;…`
pub fn format_grid(size: &Size, get: impl Fn(usize, usize) -> Option<u8>) -> String {
    let n = size.n();
    let mut rows = Vec::with_capacity(n);
    for x in 0..n {
        let mut row = String::with_capacity(n + size.block_columns());
        for y in 0..n {
            if y > 0 && y % size.bx == 0 {
                row.push(',');
            }
            row.push(match get(x, y) {
                Some(d) => char::from_digit(d as u32, 36).unwrap(),
                None => '.',
            });
        }
        rows.push(row);
    }
    rows.join(";")
}

/// Parse a grid string. Separators `,` and `;` are optional (a flat N² string
/// also parses). Returns the givens as (x, y, digit) triples.
pub fn parse_grid(size: &Size, text: &str) -> Result<Vec<(usize, usize, u8)>, ModelError> {
    let n = size.n();
    let chars: Vec<char> = text.chars().filter(|c| *c != ',' && *c != ';').collect();
    if chars.len() != n * n {
        return Err(ModelError::InvalidGrid(format!(
            "expected {} cells, got {}",
            n * n,
            chars.len()
        )));
    }
    let mut out = Vec::new();
    for (i, ch) in chars.iter().enumerate() {
        if *ch == '.' || *ch == '0' {
            continue;
        }
        match ch.to_digit(36) {
            Some(d) if d >= 1 && (d as usize) <= n => out.push((i / n, i % n, d as u8)),
            _ => {
                return Err(ModelError::InvalidGrid(format!("invalid digit '{ch}'")));
            }
        }
    }
    Ok(out)
}
