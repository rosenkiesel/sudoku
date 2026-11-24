use crate::model::game_rules::build_system;
use crate::solver::Solver;
use crate::ui::{Command, Ui, UiState, HELP, MAX_N_CONSOLE};
use std::io;

/// One game-loop step, shared by all frontends (console REPL and web UI):
/// applies a command to the state and returns an optional user-facing message.
pub fn step<S: Solver>(
    state: &mut UiState,
    solver: &S,
    command: &Command,
    max_n: usize,
) -> Option<String> {
    match command {
        Command::Solve => match state.solve_request() {
            None => Some("no puzzle yet — start with: new 3 3".to_string()),
            Some(request) => {
                let system = build_system(&request.size);
                match solver.solve(&system, &request) {
                    Ok(assignment) => {
                        state.assignment = Some(assignment);
                        Some("solved".to_string())
                    }
                    Err(e) => Some(format!("solver error: {e}")),
                }
            }
        },
        Command::Help => Some(HELP.to_string()),
        Command::Quit | Command::Show => None,
        other => match state.apply(other, max_n) {
            Ok(msg) => msg,
            Err(e) => Some(format!("error: {e}")),
        },
    }
}

/// Console game loop: depends only on the `Ui` trait, so the console can be
/// replaced by any other frontend without touching this code.
pub fn run<S: Solver, U: Ui>(mut ui: U, solver: &S) -> io::Result<()> {
    ui.note(
        "sudoku-rs | commands: new <bx> <by> | rand [seed] | set <x> <y> <n> | unset <x> <y> | \
         load <str> | solve | eps+ | eps- | view winner|probs | show | clear | help | quit",
    );
    print!("{}", ui.render()?);
    loop {
        let command = ui.next_command()?;
        if matches!(command, Command::Quit) {
            break;
        }
        if let Some(msg) = step(ui.state(), solver, &command, MAX_N_CONSOLE) {
            ui.note(&msg);
        }
        print!("{}", ui.render()?);
    }
    Ok(())
}
