use crate::model::Size;

/// Flat index of variable p(x, y, n) in a dense vector of length N^3.
pub fn idx(size: &Size, x: usize, y: usize, n: u8) -> usize {
    debug_assert!(n >= 1);
    let n_side = size.n();
    (x * n_side + y) * n_side + (n as usize - 1)
}

pub fn num_vars(size: &Size) -> usize {
    size.num_vars()
}

/// Inverse of `idx`: variable `i` corresponds to (x, y, n).
pub fn cell_digit(size: &Size, i: usize) -> (usize, usize, u8) {
    let n = size.n();
    let n_idx = i % n;
    let cell = i / n;
    let y = cell % n;
    let x = cell / n;
    (x, y, (n_idx + 1) as u8)
}

/// Iterate over all (x, y, n) triples in flat-index order.
pub fn iter(size: &Size) -> impl Iterator<Item = (usize, usize, u8)> + '_ {
    (0..size.num_vars()).map(move |i| cell_digit(size, i))
}
