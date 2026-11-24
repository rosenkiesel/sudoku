use sudoku_rs::model::{grid_indices, Size};
use sudoku_rs::ui::console::ConsoleUi;
use sudoku_rs::ui::{Command, Ui, ViewMode, DEFAULT_EPSILON, MAX_N_CONSOLE};

#[test]
fn rand_keeps_givens() {
    let mut ui = ConsoleUi::new();
    let state = ui.state();
    state
        .apply(&Command::New { bx: 2, by: 3 }, MAX_N_CONSOLE)
        .unwrap();
    state
        .apply(&Command::Set { x: 0, y: 0, n: 1 }, MAX_N_CONSOLE)
        .unwrap();
    state
        .apply(&Command::Rand { seed: Some(42) }, MAX_N_CONSOLE)
        .unwrap();
    let req = state.solve_request().unwrap();
    let s = Size::new(2, 3).unwrap();
    let t = req.target[grid_indices::idx(&s, 0, 0, 1)];
    assert!(
        t > 9.0,
        "given must stay on top of the random base (base in [-1,1] + 10), target = {t}"
    );
}

#[test]
fn load_infers_size_without_new() {
    let mut ui = ConsoleUi::new();
    ui.state()
        .apply(
            &Command::Load("1.,3.,5.;.4,.6,.2;5.,.2,.4;.3,4.,.1;4.,.1,2.;.1,.3,.5".to_string()),
            MAX_N_CONSOLE,
        )
        .unwrap();
    let req = ui.state().solve_request().unwrap();
    assert_eq!((req.size.bx, req.size.by), (2, 3));
    assert_eq!(req.target.len(), 216);
}

#[test]
fn view_mode_short_strings() {
    assert_eq!(ViewMode::Winner.as_str(), "winner");
    assert_eq!(ViewMode::Probabilities.as_str(), "probs");
    assert_eq!(ViewMode::parse("winner"), Some(ViewMode::Winner));
    assert_eq!(ViewMode::parse("probs"), Some(ViewMode::Probabilities));
    assert_eq!(ViewMode::parse("heatmap"), None); // unknown -> caller falls back to default
    assert_eq!(ViewMode::Winner.to_string(), "winner");
}

#[test]
fn eps_commands_adjust_epsilon() {
    let mut ui = ConsoleUi::new();
    let state = ui.state();
    state
        .apply(&Command::New { bx: 2, by: 3 }, MAX_N_CONSOLE)
        .unwrap();
    let e0 = state.solve_request().unwrap().epsilon;
    assert_eq!(e0, DEFAULT_EPSILON);
    state.apply(&Command::EpsMinus, MAX_N_CONSOLE).unwrap();
    let e1 = state.solve_request().unwrap().epsilon;
    state.apply(&Command::EpsPlus, MAX_N_CONSOLE).unwrap();
    let e2 = state.solve_request().unwrap().epsilon;
    assert!((e1 - e0 / 10.0).abs() < 1e-18);
    assert!((e2 - e0).abs() < 1e-18);
}
