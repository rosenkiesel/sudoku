use sudoku_rs::model::Size;
use sudoku_rs::ui::render::{conflict_cells, conflicts, winners};

#[test]
fn conflict_detection() {
    let s = Size::new(2, 3).unwrap();
    // row 0 has digit 1 twice; column 0 has digit 1 twice (rows 0 and 5)
    let wins = vec![
        1, 1, 2, 3, 4, 5, //
        2, 3, 4, 5, 6, 1, //
        3, 4, 5, 6, 1, 2, //
        4, 5, 6, 1, 2, 3, //
        5, 6, 1, 2, 3, 4, //
        1, 2, 3, 4, 5, 6,
    ];
    let (rows, cols) = conflicts(&s, &wins);
    assert_eq!(rows, vec![0]);
    assert_eq!(cols, vec![0]);
    // conflicting cells: (0,0) and (0,1) share digit 1 in row 0;
    // (0,0) and (5,0) share digit 1 in column 0
    let flags = conflict_cells(&s, &wins);
    assert!(flags[0] && flags[1] && flags[5 * 6]);
    assert_eq!(flags.iter().filter(|&&f| f).count(), 3);
}

#[test]
fn winners_argmax() {
    let s = Size::new(2, 3).unwrap();
    let mut values = vec![0.1; s.num_vars()];
    // cell (0,0): digit 4 wins; cell (1,2): digit 6 wins
    values[sudoku_rs::model::variables::idx(&s, 0, 0, 4)] = 0.9;
    values[sudoku_rs::model::variables::idx(&s, 1, 2, 6)] = 0.8;
    let wins = winners(&s, &values);
    assert_eq!(wins[0], 4);
    assert_eq!(wins[s.n() + 2], 6);
}
