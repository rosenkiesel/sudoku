use sudoku_rs::model::{grid_indices, Size};
use sudoku_rs::ui::console::ConsoleUi;
use sudoku_rs::ui::{Command, Ui, DEFAULT_EPSILON};

#[test]
fn rand_keeps_givens() {
    let mut ui = ConsoleUi::new();
    ui.apply(&Command::New { bx: 2, by: 3 }).unwrap();
    ui.apply(&Command::Set { x: 0, y: 0, n: 1 }).unwrap();
    ui.apply(&Command::Rand { seed: Some(42) }).unwrap();
    let req = ui.solve_request().unwrap();
    let s = Size::new(2, 3).unwrap();
    let t = req.target[grid_indices::idx(&s, 0, 0, 1)];
    assert!(
        t > 9.0,
        "given must stay on top of the random base (base in [-1,1] + 10), target = {t}"
    );
}

#[test]
fn eps_commands_adjust_epsilon() {
    let mut ui = ConsoleUi::new();
    ui.apply(&Command::New { bx: 2, by: 3 }).unwrap();
    let e0 = ui.solve_request().unwrap().epsilon;
    assert_eq!(e0, DEFAULT_EPSILON);
    ui.apply(&Command::EpsMinus).unwrap();
    let e1 = ui.solve_request().unwrap().epsilon;
    ui.apply(&Command::EpsPlus).unwrap();
    let e2 = ui.solve_request().unwrap().epsilon;
    assert!((e1 - e0 / 10.0).abs() < 1e-18);
    assert!((e2 - e0).abs() < 1e-18);
}
