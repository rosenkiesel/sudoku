//! HTML rendering for the web UI. Unlike the console, the web UI renders the
//! raw 0..1 values as 0..100% color-coded (no DISPLAY_SCALE).

use crate::model::grid_indices;
use crate::ui::{conflict_cells, winners, UiState, ViewMode};

const CSS: &str = r#"
body { font-family: sans-serif; margin: 1.5em; }
table.grid { border-collapse: collapse; }
td.cell { border: 1px solid #bbb; min-width: 2.2em; height: 2.2em; text-align: center;
  font-size: 1.1em; padding: 0 .2em; }
td.blockcol { border-left: 3px solid #000; }
tr.blockrow td { border-top: 3px solid #000; }
td.given { font-weight: bold; }
td.conflict { background: #ffd7d7; }
sup.mark { color: #b00; font-weight: bold; }
table.mini { border-collapse: collapse; display: inline-table; margin: 1px; }
td.mini { border: 1px solid #ddd; font-size: .65em; padding: 0 .25em; text-align: right; }
td.minicol { border-left: 2px solid #888; }
tr.minirow td { border-top: 2px solid #888; }
.note { background: #eef; border: 1px solid #99c; padding: .4em .8em; margin: .6em 0;
  white-space: pre-wrap; }
form { margin: .3em 0; }
input.cmd { width: 46em; }
"#;

pub fn page(state: &UiState, note: &str) -> String {
    let mut h = String::with_capacity(4096);
    h.push_str("<!doctype html><html><head><meta charset='utf-8'><title>sudoku-rs</title><style>");
    h.push_str(CSS);
    h.push_str("</style></head><body><h1>sudoku-rs</h1>");
    if let Some(grid) = &state.grid {
        let size = grid.size();
        h.push_str(&format!(
            "<p>size (Bx,By)=({},{}) &middot; N={} &middot; vars={} &middot; givens={} &middot; seed={} &middot; eps={:.0e} &middot; view={}</p>",
            size.bx, size.by, size.n(), size.num_vars(), grid.givens().len(), state.seed, state.epsilon, state.view
        ));
        h.push_str(&format!(
            "<p>grid: <code>{}</code></p>",
            html_escape(&grid.format())
        ));
    } else {
        h.push_str("<p>no puzzle yet — <code>new 3 3</code> or <code>load 530,070,000;…</code> (max 12×12 in the web UI)</p>");
    }
    if !note.is_empty() {
        h.push_str(&format!("<div class='note'>{}</div>", html_escape(note)));
    }
    h.push_str(
        r#"<form method="post" action="/command"><input class="cmd" name="cmd" autofocus placeholder="command, e.g. solve | view probs | load 530,070,000;600,195,000;…"><button>run</button></form>"#,
    );
    for c in [
        "solve",
        "view winner",
        "view probs",
        "eps+",
        "eps-",
        "rand 42",
        "clear",
        "help",
    ] {
        h.push_str(&format!(
            r#"<form method="post" action="/command" style="display:inline"><input type="hidden" name="cmd" value="{c}"><button>{c}</button></form>"#
        ));
    }
    if let Some(grid) = &state.grid {
        match (&state.assignment, state.view) {
            (Some(a), ViewMode::Winner) => h.push_str(&winner_table(grid, &a.values)),
            (Some(a), ViewMode::Probabilities) => h.push_str(&probs_tables(grid, &a.values)),
            _ => h.push_str(&givens_table(grid)),
        }
    }
    h.push_str("</body></html>");
    h
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn cell_class(x: usize, y: usize, size: &crate::model::Size, extra: &str) -> String {
    let mut c = String::from("cell");
    if y > 0 && y.is_multiple_of(size.bx) {
        c.push_str(" blockcol");
    }
    if !extra.is_empty() {
        c.push(' ');
        c.push_str(extra);
    }
    let _ = x;
    c
}

fn row_class(x: usize, size: &crate::model::Size) -> &'static str {
    if x > 0 && x.is_multiple_of(size.by) {
        "blockrow"
    } else {
        ""
    }
}

fn givens_table(grid: &crate::model::given_grid::GivenGrid) -> String {
    let size = grid.size();
    let n = size.n();
    let mut h = String::from("<table class='grid'>");
    for x in 0..n {
        h.push_str(&format!("<tr class='{}'>", row_class(x, &size)));
        for y in 0..n {
            let (text, extra) = match grid.get(x, y) {
                Some(d) => (d.to_string(), "given"),
                None => (".".to_string(), ""),
            };
            h.push_str(&format!(
                "<td class='{}'>{}</td>",
                cell_class(x, y, &size, extra),
                text
            ));
        }
        h.push_str("</tr>");
    }
    h.push_str("</table>");
    h
}

fn winner_table(grid: &crate::model::given_grid::GivenGrid, values: &[f64]) -> String {
    let size = grid.size();
    let n = size.n();
    let wins = winners(&size, values);
    let flags = conflict_cells(&size, &wins);
    let (rows, cols) = crate::ui::conflicts(&size, &wins);
    let mut h = String::from("<table class='grid'>");
    for x in 0..n {
        h.push_str(&format!("<tr class='{}'>", row_class(x, &size)));
        for y in 0..n {
            let d = wins[x * n + y];
            let mut marks = String::new();
            if grid.get(x, y).is_some() {
                marks.push_str("<sup class='mark'>*</sup>");
            }
            let extra = if flags[x * n + y] {
                marks.push_str("<sup class='mark'>c</sup>");
                "conflict"
            } else if grid.get(x, y).is_some() {
                "given"
            } else {
                ""
            };
            h.push_str(&format!(
                "<td class='{}'>{}{}</td>",
                cell_class(x, y, &size, extra),
                d,
                marks
            ));
        }
        h.push_str("</tr>");
    }
    h.push_str("</table>");
    h.push_str(&format!(
        "<p>conflicting rows: {:?} &middot; conflicting columns: {:?}</p>",
        rows, cols
    ));
    h
}

/// Probability view: per cell a `by × bx` mini-table, color-coded over the raw
/// 0..100% (lightness 97% → 45% as p goes 0 → 1).
fn probs_tables(grid: &crate::model::given_grid::GivenGrid, values: &[f64]) -> String {
    let size = grid.size();
    let n = size.n();
    let mut h = String::from("<table class='grid'>");
    for x in 0..n {
        h.push_str(&format!("<tr class='{}'>", row_class(x, &size)));
        for y in 0..n {
            let mut mini = String::from("<td class='cell'><table class='mini'>");
            for i in 0..size.by {
                mini.push_str(&format!(
                    "<tr class='{}'>",
                    if i > 0 { "minirow" } else { "" }
                ));
                for j in 0..size.bx {
                    let d = (i * size.bx + j + 1) as u8;
                    let p = values[grid_indices::idx(&size, x, y, d)];
                    let pct = 100.0 * p; // raw 0..100%, no DISPLAY_SCALE
                    let light = 97.0 - 52.0 * p.clamp(0.0, 1.0);
                    mini.push_str(&format!(
                        "<td class='mini{}' style='background:hsl(215 80% {:.0}%)'>{:.0}%</td>",
                        if j > 0 { " minicol" } else { "" },
                        light,
                        pct
                    ));
                }
                mini.push_str("</tr>");
            }
            mini.push_str("</table></td>");
            h.push_str(&mini);
        }
        h.push_str("</tr>");
    }
    h.push_str("</table>");
    h
}
