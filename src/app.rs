use crate::model::game_rules::build_system;
use crate::solver::Solver;
use crate::ui::{Command, Ui};
use std::io;

/// Generic game loop: depends only on the `Solver` and `Ui` traits, so the
/// console can be replaced by any other frontend without touching this code.
pub fn run<S: Solver, U: Ui>(mut ui: U, solver: &S) -> io::Result<()> {
    ui.note(
        "sudoku-rs | commands: new <bx> <by> | rand [seed] | set <x> <y> <n> | unset <x> <y> | \
         load <str> | solve | eps+ | eps- | view winner|probs | show | clear | help | quit",
    );
    ui.render()?;
    loop {
        let command = ui.next_command()?;
        match command {
            Command::Quit => break,
            Command::Solve => match ui.solve_request() {
                None => ui.note("no puzzle yet — start with: new 3 3"),
                Some(request) => {
                    let system = build_system(&request.size);
                    match solver.solve(&system, &request) {
                        Ok(assignment) => {
                            ui.note("solved");
                            ui.set_assignment(Some(assignment));
                        }
                        Err(e) => {
                            ui.note(&format!("solver error: {e}"));
                            ui.set_assignment(None);
                        }
                    }
                }
            },
            other => ui.apply(&other)?,
        }
        ui.render()?;
    }
    Ok(())
}
