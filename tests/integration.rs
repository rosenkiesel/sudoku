use sudoku_rs::model::constraints::build_system;
use sudoku_rs::model::request::SolveRequest;
use sudoku_rs::model::{variables, Size};
use sudoku_rs::solver::central_path::CentralPathSolver;
use sudoku_rs::solver::Solver;
use sudoku_rs::ui::render::{conflicts, winners};

fn solve_with_givens(size: Size, puzzle: &str, epsilon: f64) -> Vec<f64> {
    let n = size.n();
    let mut target = vec![0.0; size.num_vars()];
    for (i, ch) in puzzle.chars().enumerate() {
        if ch == '.' || ch == '0' {
            continue;
        }
        let d = ch.to_digit(10).unwrap() as u8;
        target[variables::idx(&size, i / n, i % n, d)] += 10.0;
    }
    let request = SolveRequest::new(size, target, epsilon).unwrap();
    let system = build_system(&size);
    CentralPathSolver::new()
        .solve(&system, &request)
        .unwrap()
        .values
}

// 6x6, blocks (Bx,By) = (2,3): 2x3 blocks in a 3x2 pattern
const PUZZLE_6: &str = "1.3.5..4.6.25..2.4.34..14..12..1.3.5";
const SOLUTION_6: &str = "123456345612561234234561456123612345";

// classic 9x9 (30 givens)
const PUZZLE_9: &str =
    "530070000600195000098000060800060003400803001700020006060000280000419005000080079";
const SOLUTION_9: &str =
    "534678912672195348198342567859761423426853791713924856961537284287419635345286179";

fn assert_feasible(size: Size, values: &[f64]) {
    let n = size.n();
    let system = build_system(&size);
    for (row, b) in &system.rows {
        let s: f64 = row.iter().map(|&(j, a)| a * values[j]).sum();
        assert!((s - b).abs() < 1e-6, "rule violated: |{s} - {b}| >= 1e-6");
    }
    for x in 0..n {
        for y in 0..n {
            let sum: f64 = (1..=n)
                .map(|d| values[variables::idx(&size, x, y, d as u8)])
                .sum();
            assert!((sum - 1.0).abs() < 1e-6, "cell ({x},{y}) sums to {sum}");
            assert!(
                (1..=n).all(|d| {
                    let p = values[variables::idx(&size, x, y, d as u8)];
                    (-1e-9..=1.0 + 1e-9).contains(&p)
                }),
                "probability out of [0,1] at ({x},{y})"
            );
        }
    }
}

fn assert_givens_respected(size: Size, puzzle: &str, values: &[f64]) {
    let n = size.n();
    let wins = winners(&size, values);
    for (i, ch) in puzzle.chars().enumerate() {
        if ch == '.' || ch == '0' {
            continue;
        }
        let d = ch.to_digit(10).unwrap() as u8;
        let (x, y) = (i / n, i % n);
        let p = values[variables::idx(&size, x, y, d)];
        assert!(p > 0.99, "given ({x},{y})={d} has p={p}");
        assert_eq!(wins[x * n + y], d, "winner at given ({x},{y})");
    }
}

#[test]
fn solves_6x6() {
    let size = Size::new(2, 3).unwrap();
    let values = solve_with_givens(size, PUZZLE_6, 1e-8);
    assert_feasible(size, &values);
    assert_givens_respected(size, PUZZLE_6, &values);
    let wins = winners(&size, &values);
    let sol: Vec<u8> = SOLUTION_6
        .chars()
        .map(|c| c.to_digit(10).unwrap() as u8)
        .collect();
    assert_eq!(wins, sol);
    let (rows, cols) = conflicts(&size, &wins);
    assert!(rows.is_empty() && cols.is_empty());
}

#[test]
fn solves_9x9() {
    let size = Size::new(3, 3).unwrap();
    let values = solve_with_givens(size, PUZZLE_9, 1e-8);
    assert_feasible(size, &values);
    assert_givens_respected(size, PUZZLE_9, &values);
    let wins = winners(&size, &values);
    let sol: Vec<u8> = SOLUTION_9
        .chars()
        .map(|c| c.to_digit(10).unwrap() as u8)
        .collect();
    assert_eq!(wins, sol);
    let (rows, cols) = conflicts(&size, &wins);
    assert!(rows.is_empty() && cols.is_empty());
}

#[test]
fn contradictory_givens_stay_feasible_but_conflict() {
    // same digit 1 given twice in row 0: the soft target cannot make both 1,
    // the row rule forces them to share — the winner view must flag row 0
    let size = Size::new(2, 3).unwrap();
    let mut target = vec![0.0; size.num_vars()];
    target[variables::idx(&size, 0, 0, 1)] += 10.0;
    target[variables::idx(&size, 0, 1, 1)] += 10.0;
    let request = SolveRequest::new(size, target, 1e-8).unwrap();
    let system = build_system(&size);
    let values = CentralPathSolver::new()
        .solve(&system, &request)
        .unwrap()
        .values;
    assert_feasible(size, &values);
    let p00 = values[variables::idx(&size, 0, 0, 1)];
    let p01 = values[variables::idx(&size, 0, 1, 1)];
    assert!(
        (p00 - 0.5).abs() < 0.1 && (p01 - 0.5).abs() < 0.1,
        "p00={p00} p01={p01}"
    );
    let wins = winners(&size, &values);
    let (rows, _cols) = conflicts(&size, &wins);
    assert!(rows.contains(&0), "row 0 must be flagged, got {rows:?}");
}
