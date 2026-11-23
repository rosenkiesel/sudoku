use crate::model::constraints::build_system;
use crate::solver::Solver;
use crate::ui::{Command, Ui};
use std::io;

pub const DEFAULT_EPSILON: f64 = 1e-6;

/// Generic game loop: depends only on the `Solver` and `Ui` traits, so the
/// console can be replaced by any other frontend without touching this code.
pub fn run<S: Solver, U: Ui>(mut ui: U, solver: &S) -> io::Result<()> {
    ui.note(
        "sudoku-rs | commands: new <bx> <by> | rand [seed] | set <x> <y> <n> | unset <x> <y> | \
         load <str> | solve [eps] | view winner|probs | show | clear | quit",
    );
    ui.render()?;
    loop {
        let command = ui.next_command()?;
        match command {
            Command::Quit => break,
            Command::Solve { epsilon } => {
                let eps = epsilon.unwrap_or(DEFAULT_EPSILON);
                match ui.solve_request(eps) {
                    None => ui.note("no puzzle yet — start with: new 3 3"),
                    Some(request) => {
                        let system = build_system(&request.size);
                        match solver.solve(&system, &request) {
                            Ok(assignment) => {
                                ui.note(&format!("solved: central path down to mu = {eps:e}"));
                                ui.set_assignment(Some(assignment));
                            }
                            Err(e) => {
                                ui.note(&format!("solver error: {e}"));
                                ui.set_assignment(None);
                            }
                        }
                    }
                }
            }
            other => ui.apply(&other)?,
        }
        ui.render()?;
    }
    Ok(())
}
