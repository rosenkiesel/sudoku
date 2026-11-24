use sudoku_rs::app;
use sudoku_rs::solver::central_path::CentralPathSolver;
use sudoku_rs::ui::console::ConsoleUi;

fn main() {
    if std::env::args().nth(1).as_deref() == Some("web") {
        #[cfg(feature = "web")]
        {
            if let Err(e) = sudoku_rs::ui::web::serve() {
                eprintln!("fatal: {e}");
                std::process::exit(1);
            }
            return;
        }
        #[cfg(not(feature = "web"))]
        {
            eprintln!(
                "built without the `web` feature — run: cargo run --no-default-features -- web"
            );
            eprintln!("or with web UI:                            cargo run -- web");
            std::process::exit(1);
        }
    }
    let ui = ConsoleUi::new();
    let solver = CentralPathSolver::new();
    if let Err(e) = app::run(ui, &solver) {
        eprintln!("fatal: {e}");
        std::process::exit(1);
    }
}
