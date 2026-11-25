// sudoku-rs web UI client (REQUIREMENTS §6/§7).
// wasm available  -> decode cookies, compute, render and re-encode cookies locally
//                   (same Rust cookie codec, byte-identical cookies)
// wasm missing    -> every command goes to POST /api/command (server runs the
//                   same Rust step and returns state + cookie pairs)
"use strict";

let wasm = null;          // wasm module (cookie_decode/cookie_encode/rand_base/solve) or null
let state = null;         // StateJson: {grid, eps, view, base, start, seeds, note}
let solveResult = null;   // {values, winners, conflict_rows, conflict_cols} from last solve

async function boot() {
  if (typeof WebAssembly === "object") {
    try {
      const mod = await import("/pkg/sudoku_rs.js"); // wasm-pack --target web output
      await mod.default();
      wasm = mod;
    } catch (e) { wasm = null; }
  }
  document.getElementById("randbtn").textContent =
    "rand (new time seed)" + (wasm ? "" : " [server mode]");
  await loadState();
  bindUi();
  render();
}

function cookiePairs() {
  if (!document.cookie) return [];
  return document.cookie.split(";").map(p => {
    const i = p.indexOf("=");
    return [p.slice(0, i).trim(), p.slice(i + 1).trim()];
  });
}

function writeCookies(pairs) {
  for (const [name, value] of pairs) {
    document.cookie = `${name}=${value}; Path=/; SameSite=Lax; Max-Age=2592000`;
  }
}

async function loadState() {
  if (wasm) {
    state = wasm.cookie_decode(cookiePairs());
  } else {
    const r = await fetch("/api/command", {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ cmd: "show", cookies: cookiePairs() }),
    });
    const j = await r.json();
    writeCookies(j.cookies);
    state = j.state;
  }
}

function persist(note) {
  state.note = note || "";
  const pairs = wasm ? wasm.cookie_encode(state) : null;
  if (pairs) { writeCookies(pairs); return pairs; }
  return cookiePairs(); // server mode: cookies written after /api/command
}

// ---- grid string helpers -------------------------------------------------
function parseGrid(s) {
  const rows = s.split(";");
  const n = rows.length;
  const segs = rows[0].split(",");
  const bx = segs[0].length, by = segs.length;
  const cells = [];
  for (let x = 0; x < n; x++) {
    const row = [];
    for (const seg of rows[x].split(",")) for (const ch of seg) {
      row.push(ch === "." || ch === "0" ? 0 : parseInt(ch, 36));
    }
    cells.push(row);
  }
  return { n, bx, by, cells };
}
function formatGrid(g) {
  const rows = [];
  for (let x = 0; x < g.n; x++) {
    const segs = [];
    for (let c = 0; c < g.by; c++) {
      let seg = "";
      for (let j = 0; j < g.bx; j++) {
        const d = g.cells[x][c * g.bx + j];
        seg += d ? d.toString(36) : ".";
      }
      segs.push(seg);
    }
    rows.push(segs.join(","));
  }
  return rows.join(";");
}

// ---- commands -------------------------------------------------------------
async function runCommand(cmd) {
  if (wasm) {
    let note = "";
    const s = state;
    if (cmd.startsWith("new ")) {
      const [, bx, by] = cmd.split(/\s+/);
      const n = (+bx) * (+by);
      if (n > 12) { note = "error: web UI supports up to 12x12"; }
      else {
        s.grid = Array.from({ length: n }, () => ".".repeat(n)).join(";").replaceAll(".;", ";,");
        // build proper grid string with block separators:
        s.grid = gridString(+bx, +by, null);
        s.base = null; s.start = null; solveResult = null;
      }
    } else if (cmd.startsWith("load ")) {
      s.grid = cmd.slice(5); s.start = null; solveResult = null;
      const g = parseGrid(s.grid); if (g.n > 12) { note = "error: web UI supports up to 12x12"; s.grid = null; }
    } else if (cmd.startsWith("set ")) {
      const [, x, y, d] = cmd.split(/\s+/);
      const g = parseGrid(s.grid); g.cells[+x][+y] = parseInt(d, 10);
      s.grid = formatGrid(g); s.start = null; solveResult = null;
    } else if (cmd.startsWith("unset ")) {
      const [, x, y] = cmd.split(/\s+/);
      const g = parseGrid(s.grid); g.cells[+x][+y] = 0;
      s.grid = formatGrid(g); s.start = null; solveResult = null;
    } else if (cmd === "clear") {
      const g = parseGrid(s.grid);
      for (const row of g.cells) row.fill(0);
      s.grid = formatGrid(g); s.start = null; solveResult = null;
    } else if (cmd === "rand" || cmd.startsWith("rand ")) {
      const seed = cmd === "rand" ? Date.now() : Number(cmd.slice(5));
      const g = parseGrid(s.grid);
      s.base = wasm.rand_base(seed, g.n ** 3);
      s.start = null; solveResult = null;
      s.seeds = [seed, ...s.seeds.filter(x => x !== seed)].slice(0, 8);
      note = "rand seed=" + seed;
    } else if (cmd === "eps+") { s.eps = Math.min(s.eps * 10, 1.0); note = "epsilon = " + s.eps; }
    else if (cmd === "eps-") { s.eps = Math.max(s.eps / 10, 1e-12); note = "epsilon = " + s.eps; }
    else if (cmd === "view winner") s.view = "winner";
    else if (cmd === "view probs") s.view = "probs";
    else if (cmd === "solve") {
      if (!s.grid) { note = "no puzzle yet — start with: new 3 3"; }
      else {
        const g = parseGrid(s.grid);
        solveResult = wasm.solve(s.grid, s.eps, s.seeds[0] ?? null, s.base, s.start);
        s.start = solveResult.values; note = "solved";
      }
    } else { note = "unknown command: " + cmd; }
    persist(note);
  } else {
    const r = await fetch("/api/command", {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ cmd, cookies: cookiePairs() }),
    });
    const j = await r.json();
    writeCookies(j.cookies);
    state = j.state;
    if (cmd === "solve") {
      // server solved; reconstruct winners client-side from state.start
      const g = parseGrid(state.grid);
      solveResult = computeWinners(g, state.start);
    }
  }
  render();
}

function gridString(bx, by, cells) {
  const n = bx * by;
  const rows = [];
  for (let x = 0; x < n; x++) {
    const segs = [];
    for (let c = 0; c < by; c++) {
      let seg = "";
      for (let j = 0; j < bx; j++) {
        const d = cells ? cells[x][c * bx + j] : 0;
        seg += d ? d.toString(36) : ".";
      }
      segs.push(seg);
    }
    rows.push(segs.join(","));
  }
  return rows.join(";");
}

function computeWinners(g, values) {
  const n = g.n, wins = [];
  const conflict = [];
  for (let x = 0; x < n; x++) for (let y = 0; y < n; y++) {
    let best = 1, bp = -1;
    for (let d = 1; d <= n; d++) {
      const p = values[(x * n + y) * n + (d - 1)];
      if (p > bp) { bp = p; best = d; }
    }
    wins.push(best); conflict.push(false);
  }
  for (let x = 0; x < n; x++) {
    const cnt = {};
    for (let y = 0; y < n; y++) cnt[wins[x * n + y]] = (cnt[wins[x * n + y]] || 0) + 1;
    for (let y = 0; y < n; y++) if (cnt[wins[x * n + y]] > 1) conflict[x * n + y] = true;
  }
  for (let y = 0; y < n; y++) {
    const cnt = {};
    for (let x = 0; x < n; x++) cnt[wins[x * n + y]] = (cnt[wins[x * n + y]] || 0) + 1;
    for (let x = 0; x < n; x++) if (cnt[wins[x * n + y]] > 1) conflict[x * n + y] = true;
  }
  const rows = [], cols = [];
  for (let x = 0; x < n; x++) { const c = {}; for (let y = 0; y < n; y++) c[wins[x*n+y]] = 1; if (Object.keys(c).length < n) rows.push(x); }
  for (let y = 0; y < n; y++) { const c = {}; for (let x = 0; x < n; x++) c[wins[x*n+y]] = 1; if (Object.keys(c).length < n) cols.push(y); }
  return { values, winners: wins, conflict_rows: rows, conflict_cols: cols, conflict };
}

// ---- rendering ------------------------------------------------------------
function esc(s) { return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;"); }

function render() {
  const s = state;
  let info = "no puzzle yet — new 3 3 or load 530,070,000;… (web UI max 12x12)";
  let gridHtml = "";
  if (s.grid) {
    const g = parseGrid(s.grid);
    info = `size (Bx,By)=(${g.bx},${g.by}) · N=${g.n} · givens=${g.cells.flat().filter(d => d).length} · eps=${s.eps} · view=${s.view}`;
    gridHtml = renderGrid(g, s);
  }
  document.getElementById("info").textContent = info + (wasm ? " · wasm: local" : " · wasm: off (server)");
  const noteEl = document.getElementById("note");
  if (s.note) { noteEl.style.display = ""; noteEl.textContent = s.note; } else noteEl.style.display = "none";
  document.getElementById("app").innerHTML = gridHtml;
  const sel = document.getElementById("seeds");
  sel.innerHTML = (s.seeds || []).map(sd => `<option value="${sd}">${sd}</option>`).join("");
}

function renderGrid(g, s) {
  const n = g.n;
  const useSolve = solveResult && s.start;
  let h = "<table class='grid'>";
  for (let x = 0; x < n; x++) {
    h += `<tr class='${x > 0 && x % g.by === 0 ? "blockrow" : ""}'>`;
    for (let y = 0; y < n; y++) {
      const bc = y > 0 && y % g.bx === 0 ? " blockcol" : "";
      const d = g.cells[x][y];
      if (useSolve && (s.view === "winner")) {
        const w = solveResult.winners[x * n + y];
        const conf = solveResult.conflict[x * n + y];
        let marks = (d ? "<sup class='mark'>*</sup>" : "") + (conf ? "<sup class='mark'>c</sup>" : "");
        h += `<td class='cell${bc}${d ? " given" : ""}${conf ? " conflict" : ""}'>${w}${marks}</td>`;
      } else if (useSolve && s.view === "probs") {
        let mini = "<table class='mini'>";
        for (let i = 0; i < g.by; i++) {
          mini += `<tr class='${i > 0 ? "minirow" : ""}'>`;
          for (let j = 0; j < g.bx; j++) {
            const d2 = i * g.bx + j + 1;
            const p = s.start[(x * n + y) * n + (d2 - 1)];
            const light = 97 - 52 * Math.min(1, Math.max(0, p));
            mini += `<td class='mini${j > 0 ? " minicol" : ""}" style='background:hsl(215 80% ${light.toFixed(0)}%)'>${(100 * p).toFixed(0)}%</td>`;
          }
          mini += "</tr>";
        }
        h += `<td class='cell${bc}'>${mini}</table></td>`;
      } else {
        h += `<td class='cell${bc}${d ? " given" : ""}'>${d || "."}</td>`;
      }
    }
    h += "</tr>";
  }
  h += "</table>";
  if (useSolve && s.view === "winner") {
    h += `<p>conflicting rows: ${JSON.stringify(solveResult.conflict_rows)} · conflicting columns: ${JSON.stringify(solveResult.conflict_cols)}</p>`;
  }
  return h;
}

// ---- interaction (REQUIREMENTS §7) ----------------------------------------
function bindUi() {
  document.getElementById("cmdform").addEventListener("submit", e => {
    e.preventDefault();
    const c = document.getElementById("cmd").value.trim();
    if (c) runCommand(c);
    document.getElementById("cmd").value = "";
  });
  document.getElementById("solvebtn").addEventListener("click", () => runCommand("solve"));
  document.getElementById("viewwinner").addEventListener("click", () => runCommand("view winner"));
  document.getElementById("viewprobs").addEventListener("click", () => runCommand("view probs"));
  document.getElementById("randform").addEventListener("submit", e => {
    e.preventDefault(); runCommand("rand");
  });
  document.getElementById("seeds").addEventListener("change", e => {
    if (e.target.value) runCommand("rand " + e.target.value);
  });
  // cell clicks: event delegation on the grid container
  document.getElementById("app").addEventListener("click", e => {
    const td = e.target.closest("td.cell");
    if (!td || !state.grid) return;
    const tr = td.parentElement, table = tr.parentElement;
    const x = Array.from(table.rows).indexOf(tr);
    const cells = Array.from(tr.children);
    const y = cells.indexOf(td);
    if (x < 0 || y < 0) return;
    const g = parseGrid(state.grid);
    if (g.cells[x][y]) runCommand(`unset ${x} ${y}`);       // 1) given -> delete
    else showDigitPopup(x, y);                               // 2) empty -> popup
  });
}

function showDigitPopup(x, y) {
  const g = parseGrid(state.grid);
  const pop = document.getElementById("popup");
  pop.innerHTML = "";
  for (let d = 1; d <= g.n; d++) {
    const b = document.createElement("button");
    b.textContent = d;
    b.addEventListener("click", () => { pop.style.display = "none"; runCommand(`set ${x} ${y} ${d}`); });
    pop.appendChild(b);
  }
  const c = document.createElement("button");
  c.textContent = "✕";
  c.addEventListener("click", () => { pop.style.display = "none"; });
  pop.appendChild(c);
  pop.style.display = "";
}

boot();
