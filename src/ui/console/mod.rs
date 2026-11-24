pub mod render;

use crate::ui::{parse_command, Command, Ui, UiState, HELP};
use std::io::{self, BufRead, Write};

/// Console frontend: a REPL around the shared `UiState`.
pub struct ConsoleUi {
    state: UiState,
}

impl ConsoleUi {
    pub fn new() -> Self {
        Self {
            state: UiState::new(),
        }
    }
}

impl Default for ConsoleUi {
    fn default() -> Self {
        Self::new()
    }
}

impl Ui for ConsoleUi {
    fn state(&mut self) -> &mut UiState {
        &mut self.state
    }

    fn next_command(&mut self) -> io::Result<Command> {
        let stdin = io::stdin();
        loop {
            print!("> ");
            io::stdout().flush()?;
            let mut line = String::new();
            if stdin.lock().read_line(&mut line)? == 0 {
                return Ok(Command::Quit);
            }
            match parse_command(line.trim()) {
                Ok(command) => return Ok(command),
                Err(message) => self.note(&format!("error: {message}")),
            }
        }
    }

    fn note(&mut self, message: &str) {
        println!("{message}");
    }

    fn help_text(&self) -> &'static str {
        HELP
    }

    fn render(&mut self) -> io::Result<String> {
        let mut buf: Vec<u8> = Vec::new();
        {
            let w = &mut buf;
            let state = &self.state;
            let Some(grid) = &state.grid else {
                writeln!(w, "(no puzzle yet — start with: new 3 3)")?;
                let s = String::from_utf8(buf).unwrap();
                print!("{s}");
                return Ok(s);
            };
            let size = grid.size();
            writeln!(
                w,
                "size (Bx,By)=({},{})  N={}  vars={}  givens={}  seed={}  eps={:.0e}",
                size.bx,
                size.by,
                size.n(),
                size.num_vars(),
                grid.givens().len(),
                state.seed,
                state.epsilon
            )?;
            match (&state.assignment, state.view) {
                (Some(a), crate::ui::ViewMode::Winner) => {
                    render::render_winner(&mut *w, grid, &a.values)?
                }
                (Some(a), crate::ui::ViewMode::Probabilities) => {
                    render::render_probs(&mut *w, &size, &a.values)?
                }
                _ => render::render_grid(&mut *w, &size, 2, |x, y| match grid.get(x, y) {
                    Some(d) => format!("{d} "),
                    None => ". ".to_string(),
                })?,
            }
            writeln!(w, "grid: {grid}")?;
        }
        let s = String::from_utf8(buf).map_err(io::Error::other)?;
        print!("{s}");
        Ok(s)
    }
}
