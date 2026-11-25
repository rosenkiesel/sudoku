//! HTML shell for the web UI. The grid itself is rendered client-side by
//! `/static/app.js` (wasm mode: fully local; fallback: via `POST /api/command`).

pub const CSS: &str = r#"
body { font-family: sans-serif; margin: 1.5em; }
table.grid { border-collapse: collapse; }
td.cell { border: 1px solid #bbb; min-width: 2.2em; height: 2.2em; text-align: center;
  font-size: 1.1em; padding: 0 .2em; cursor: pointer; }
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
#popup { position: fixed; background: #fff; border: 2px solid #000; padding: .5em; }
#popup button { font-size: 1.2em; margin: 2px; width: 2em; height: 2em; }
"#;

/// The HTML shell: header, command form, container for the JS-rendered grid.
pub fn shell() -> String {
    format!(
        r#"<!doctype html><html><head><meta charset='utf-8'><title>sudoku-rs</title>
<style>{CSS}</style></head><body>
<h1>sudoku-rs</h1>
<p id="info"></p>
<div class="note" id="note" style="display:none"></div>
<form id="cmdform"><input class="cmd" id="cmd" autofocus
  placeholder="command, e.g. new 3 3 | load 530,070,000;600,195,000;… | solve | view probs | eps-">
  <button>run</button></form>
<form id="randform" style="display:inline">
  <button type="submit" id="randbtn">rand (new time seed)</button>
  <select id="seeds"></select>
</form>
<button id="viewwinner">view winner</button>
<button id="viewprobs">view probs</button>
<button id="solvebtn">solve</button>
<div id="app"></div>
<div id="popup" style="display:none"></div>
<noscript><p>this page needs JavaScript (wasm optional; without wasm the server computes via /api/command)</p></noscript>
<script src="/static/app.js"></script>
</body></html>"#
    )
}
