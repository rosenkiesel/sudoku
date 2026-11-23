use sudoku_rs::app;
use sudoku_rs::solver::central_path::CentralPathSolver;
use sudoku_rs::ui::console::ConsoleUi;

fn main() {
    let ui = ConsoleUi::new();
    let solver = CentralPathSolver::new();
    if let Err(e) = app::run(ui, &solver) {
        eprintln!("fatal: {e}");
        std::process::exit(1);
    }
}
