//! HTML rendering for the web UI. `page()` renders the **full page
//! server-side** from the cookie state, so the UI works entirely without
//! JavaScript (forms POST to `/command`). `static/app.js` is progressive
//! enhancement: with wasm available it takes over (local render + cookie
//! updates + §7 mouse interaction).

use crate::model::given_grid::GivenGrid;
use crate::model::{grid_indices, Size};
use crate::ui::{conflict_cells, conflicts, winners, UiState, ViewMode};

pub const CSS: &str = r#"
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
form { margin: .3em 0; display: inline-block; }
input.cmd { width: 40em; }
#popup { position: fixed; background: #fff; border: 2px solid #000; padding: .5em; }
#popup button { font-size: 1.2em; margin: 2px; width: 2em; height: 2em; }
"#;

/// Full server-rendered page (no JS required): info, note, command form,
/// buttons, seed history, and the grid (givens / winner / probs).
pub fn page(state: &UiState, note: &str) -> String {
    let mut h = String::with_capacity(8192);
    h.push_str("<!doctype html><html><head><meta charset='utf-8'><title>sudoku-rs</title><style>");
    h.push_str(CSS);
    h.push_str("</style></head><body><h1>sudoku-rs</h1>");
    match &state.grid {
        Some(grid) => {
            let size = grid.size();
            h.push_str(&format!(
                "<p>size (Bx,By)=({},{}) &middot; N={} &middot; vars={} &middot; givens={} &middot; seed={} &middot; eps={:.0e} &middot; view={}</p>",
                size.bx, size.by, size.n(), size.num_vars(), grid.givens().len(), state.seed, state.epsilon, state.view
            ));
            h.push_str(&format!("<p>grid: <code>{}</code></p>", esc(&grid.format())));
        }
        None => h.push_str("<p>no puzzle yet — <code>new 3 3</code> or <code>load 530,070,000;…</code> (web UI max 12x12)</p>"),
    }
    if !note.is_empty() {
        h.push_str(&format!("<div class='note'>{}</div>", esc(note)));
    }
    h.push_str(
        r#"<form method="post" action="/command"><input class="cmd" name="cmd" autofocus placeholder="command: new 3 3 | load 530,070,000;600,195,000;… | solve | view probs | eps- | set 0 0 5 | unset 0 0 | rand [seed]"><button>run</button></form>"#,
    );
    for c in [
        "solve",
        "view winner",
        "view probs",
        "eps+",
        "eps-",
        "rand",
        "clear",
        "help",
    ] {
        h.push_str(&format!(
            r#"<form method="post" action="/command"><input type="hidden" name="cmd" value="{c}"><button>{c}</button></form>"#
        ));
    }
    if !state.seeds.is_empty() {
        h.push_str("<p>recent seeds: ");
        for s in state.seeds.iter().take(crate::ui::MAX_SEEDS) {
            h.push_str(&format!(
                r#"<form method="post" action="/command"><input type="hidden" name="cmd" value="rand {s}"><button>{s}</button></form>"#
            ));
        }
        h.push_str("</p>");
    }
    if let Some(grid) = &state.grid {
        match (&state.assignment, state.view) {
            (Some(a), ViewMode::Winner) => h.push_str(&winner_table(grid, &a.values)),
            (Some(a), ViewMode::Probabilities) => h.push_str(&probs_tables(grid, &a.values)),
            _ => h.push_str(&givens_table(grid)),
        }
    }
    h.push_str("<div id='app'></div><div id='popup' style='display:none'></div>");
    h.push_str("<noscript><p>(JavaScript off — fully functional via the forms above; wasm/JS only adds mouse interaction and client-side solving)</p></noscript>");
    h.push_str(r#"<script src="/static/app.js"></script></body></html>"#);
    h
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn givens_table(grid: &GivenGrid) -> String {
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
                "<td class='cell{} {}'>{}</td>",
                col_class(y, &size),
                extra,
                text
            ));
        }
        h.push_str("</tr>");
    }
    h.push_str("</table>");
    h
}

fn winner_table(grid: &GivenGrid, values: &[f64]) -> String {
    let size = grid.size();
    let n = size.n();
    let wins = winners(&size, values);
    let flags = conflict_cells(&size, &wins);
    let (rows, cols) = conflicts(&size, &wins);
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
                "<td class='cell{} {}'>{}{}</td>",
                col_class(y, &size),
                extra,
                d,
                marks
            ));
        }
        h.push_str("</tr>");
    }
    h.push_str("</table>");
    h.push_str(&format!(
        "<p>conflicting rows: {rows:?} &middot; conflicting columns: {cols:?}</p>"
    ));
    h
}

/// Probability view: per cell a `by × bx` mini-table, color-coded over the raw
/// 0..100% (lightness 97% → 45% as p goes 0 → 1).
fn probs_tables(grid: &GivenGrid, values: &[f64]) -> String {
    let size = grid.size();
    let n = size.n();
    let mut h = String::from("<table class='grid'>");
    for x in 0..n {
        h.push_str(&format!("<tr class='{}'>", row_class(x, &size)));
        for y in 0..n {
            let mut mini = String::from("<table class='mini'>");
            for i in 0..size.by {
                mini.push_str(&format!(
                    "<tr class='{}'>",
                    if i > 0 { "minirow" } else { "" }
                ));
                for j in 0..size.bx {
                    let d = (i * size.bx + j + 1) as u8;
                    let p = values[grid_indices::idx(&size, x, y, d)];
                    let light = 97.0 - 52.0 * p.clamp(0.0, 1.0);
                    mini.push_str(&format!(
                        "<td class='mini{}' style='background:hsl(215 80% {:.0}%)'>{:.0}%</td>",
                        if j > 0 { " minicol" } else { "" },
                        light,
                        100.0 * p
                    ));
                }
                mini.push_str("</tr>");
            }
            mini.push_str("</table>");
            h.push_str(&format!(
                "<td class='cell{}'>{mini}</td>",
                col_class(y, &size)
            ));
        }
        h.push_str("</tr>");
    }
    h.push_str("</table>");
    h
}

fn col_class(y: usize, size: &Size) -> &'static str {
    if y > 0 && crate::model::is_multiple_of(y, size.bx) {
        "blockcol"
    } else {
        ""
    }
}

fn row_class(x: usize, size: &Size) -> &'static str {
    if x > 0 && crate::model::is_multiple_of(x, size.by) {
        "blockrow"
    } else {
        ""
    }
}
