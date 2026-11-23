use sudoku_rs::model::constraints::{block_of, build_system};
use sudoku_rs::model::request::SolveRequest;
use sudoku_rs::model::{variables, Size};

#[test]
fn size_params() {
    let s9 = Size::new(3, 3).unwrap();
    assert_eq!(s9.n(), 9);
    assert_eq!(s9.num_vars(), 729);
    assert_eq!(s9.block_columns(), 3);
    assert_eq!(s9.block_rows(), 3);

    let s6 = Size::new(2, 3).unwrap();
    assert_eq!(s6.n(), 6);
    assert_eq!(s6.num_vars(), 216);
    assert_eq!(s6.block_columns(), 3);
    assert_eq!(s6.block_rows(), 2);

    assert!(Size::new(0, 3).is_err());
    assert!(Size::new(3, 0).is_err());
}

#[test]
fn index_roundtrip() {
    let s = Size::new(2, 3).unwrap();
    for (x, y, d) in variables::iter(&s) {
        assert_eq!(
            variables::cell_digit(&s, variables::idx(&s, x, y, d)),
            (x, y, d)
        );
    }
}

#[test]
fn system_counts_and_bounds() {
    for (bx, by) in [(3, 3), (2, 3)] {
        let s = Size::new(bx, by).unwrap();
        let sys = build_system(&s);
        assert_eq!(sys.num_vars, s.num_vars());
        assert_eq!(sys.num_rows(), 4 * s.n() * s.n());
        assert!(sys.lower.iter().all(|&v| v == 0.0));
        assert!(sys.upper.iter().all(|&v| v == 1.0));
        // every rule row sums N coefficients; x = 1/N is feasible for each
        let n = s.n() as f64;
        for (row, b) in &sys.rows {
            let sum: f64 = row.iter().map(|&(_, a)| a).sum();
            assert!((sum / n - b).abs() < 1e-12);
            assert_eq!(row.len(), s.n());
        }
    }
}

#[test]
fn block_coverage() {
    let s = Size::new(2, 3).unwrap();
    let n = s.n();
    for x in 0..n {
        for y in 0..n {
            let (cx, cy) = block_of(&s, x, y);
            assert!(cx < s.block_columns());
            assert!(cy < s.block_rows());
        }
    }
    assert_eq!(block_of(&s, 0, 0), (0, 0));
    assert_eq!(block_of(&s, 2, 1), (0, 0));
    assert_eq!(block_of(&s, 3, 0), (0, 1));
    assert_eq!(block_of(&s, 0, 2), (1, 0));
    assert_eq!(block_of(&s, 5, 5), (2, 1));
}

#[test]
fn request_validation() {
    let s = Size::new(2, 3).unwrap();
    assert!(SolveRequest::new(s, vec![0.0; 216], 1e-6).is_ok());
    assert!(SolveRequest::new(s, vec![0.0; 10], 1e-6).is_err());
    assert!(SolveRequest::new(s, vec![0.0; 216], 0.0).is_err());
    assert!(SolveRequest::new(s, vec![0.0; 216], -1.0).is_err());
    assert!(SolveRequest::new(s, vec![0.0; 216], f64::NAN).is_err());
}
