use sudoku_rs::model::game_rules::{block_of, build_system};
use sudoku_rs::model::request::SolveRequest;
use sudoku_rs::model::{grid_indices, Size};

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
    for (x, y, d) in grid_indices::iter(&s) {
        assert_eq!(
            grid_indices::cell_digit(&s, grid_indices::idx(&s, x, y, d)),
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
fn given_grid_string_format() {
    use sudoku_rs::model::given_grid::GivenGrid;
    let s = Size::new(2, 3).unwrap();
    let text = "1.,3.,5.;.4,.6,.2;5.,.2,.4;.3,4.,.1;4.,.1,2.;.1,.3,.5";
    let grid = GivenGrid::parse(&s, text).unwrap();
    assert_eq!(grid.dimensions(), (2, 3));
    assert_eq!(grid.size(), s);
    assert_eq!(grid.givens().len(), 18);
    // flat string (no separators) parses to the same grid; format roundtrip
    let flat = "1.3.5..4.6.25..2.4.34..14..12..1.3.5";
    assert_eq!(GivenGrid::parse(&s, flat).unwrap().format(), grid.format());
    assert_eq!(grid.format(), text);
    assert_eq!(grid.to_string(), text);
    assert!(GivenGrid::parse(&s, "1,2").is_err());
    // set/unset/clear/get
    let mut g = GivenGrid::new(s);
    g.set(0, 0, 3).unwrap();
    assert_eq!(g.get(0, 0), Some(3));
    assert!(g.set(0, 0, 9).is_err()); // digit out of range for N=6
    g.unset(0, 0);
    assert_eq!(g.get(0, 0), None);

    let s9 = Size::new(3, 3).unwrap();
    let t9 = "530,070,000;600,195,000;098,000,060;800,060,003;400,803,001;700,020,006;060,000,280;000,419,005;000,080,079";
    assert_eq!(GivenGrid::parse(&s9, t9).unwrap().givens().len(), 30);
}

#[test]
fn given_grid_infers_size_from_string() {
    use sudoku_rs::model::given_grid::GivenGrid;
    // separators give N (rows) and (Bx, By): 6 rows, 3 block columns of width 2
    let g = GivenGrid::parse_auto("1.,3.,5.;.4,.6,.2;5.,.2,.4;.3,4.,.1;4.,.1,2.;.1,.3,.5").unwrap();
    assert_eq!(g.dimensions(), (2, 3));
    assert_eq!(g.givens().len(), 18);

    let t9 = "530,070,000;600,195,000;098,000,060;800,060,003;400,803,001;700,020,006;060,000,280;000,419,005;000,080,079";
    assert_eq!(GivenGrid::parse_auto(t9).unwrap().dimensions(), (3, 3));

    // flat N^2 string without separators: square grid when N is a perfect square
    let flat9 = "530070000600195000098000060800060003400803001700020006060000280000419005000080079";
    assert_eq!(GivenGrid::parse_auto(flat9).unwrap().dimensions(), (3, 3));

    // inconsistent separators / non-square flat string -> error
    assert!(GivenGrid::parse_auto("1.,2.;3.4").is_err());
    assert!(GivenGrid::parse_auto("123456").is_err());
}

#[test]
fn request_validation() {
    use sudoku_rs::model::request::default_start;
    let s = Size::new(2, 3).unwrap();
    assert!(SolveRequest::new(s, vec![0.0; 216], 1e-6).is_ok());
    assert!(SolveRequest::new(s, vec![0.0; 10], 1e-6).is_err());
    assert!(SolveRequest::new(s, vec![0.0; 216], 0.0).is_err());
    assert!(SolveRequest::new(s, vec![0.0; 216], -1.0).is_err());
    assert!(SolveRequest::new(s, vec![0.0; 216], f64::NAN).is_err());
    // default start: 1/(bx*by) = 1/6 in every variable
    assert_eq!(default_start(&s), vec![1.0 / 6.0; 216]);
    let req = SolveRequest::new(s, vec![0.0; 216], 1e-6).unwrap();
    assert_eq!(req.start, vec![1.0 / 6.0; 216]);
    // explicit start must be length N^3 and finite; boundary values are
    // clamped into the open interval (a previous result may contain 0.0/1.0)
    assert!(SolveRequest::with_start(s, vec![0.0; 216], 1e-6, vec![0.5; 216]).is_ok());
    assert!(SolveRequest::with_start(s, vec![0.0; 216], 1e-6, vec![0.5; 10]).is_err());
    let clamped = SolveRequest::with_start(s, vec![0.0; 216], 1e-6, vec![1.0; 216]).unwrap();
    assert!(clamped.start.iter().all(|&v| v > 0.0 && v < 1.0));
    assert!(SolveRequest::with_start(s, vec![0.0; 216], 1e-6, vec![f64::NAN; 216]).is_err());
}
