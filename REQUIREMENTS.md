# REQUIREMENTS.md

Detailed specifications that do not belong in the README.

- [README.md](README.md) — overview and idea
- [ARCHITECTURE.md](ARCHITECTURE.md) — layering and the linear model
- this file — concrete interface behavior and numerics

## 1. Console interface (v1)

REPL with a `>` prompt on stdin/stdout. Unknown or invalid input prints an error and the
REPL continues.

| Command | Effect |
|---|---|
| `new <bx> <by>` | start a new puzzle of size `(Bx, By)` (N = Bx·By); resets givens, target base and solution |
| `rand [seed]` | fill the **entire target base** with uniform random values in **[-1, +1]**; optional seed for reproducibility; givens are kept |
| `set <x> <y> <n>` | add a given: `target[idx(x,y,n)] += 10.0`, on top of the current value (e.g. the random base). At most one given per cell; setting an occupied cell replaces the old given |
| `unset <x> <y>` | remove the given at cell `(x, y)` |
| `solve` | build `LinearSystem` + `SolveRequest` and run the central-path solver down to the **current** epsilon |
| `eps+` | multiply the current epsilon by 10 (max 1.0) |
| `eps-` | divide the current epsilon by 10 (min 1e-12) |
| `view winner` | winner view (§4.1) |
| `view probs` | probability view (§4.2) |
| `show` | re-render: grid, grid string and header |
| `load <string>` | grid in string format (§1.1); each digit becomes a given. Without a current puzzle, the size `(Bx,By)` is inferred from the string's separators |
| `clear` | remove all givens (the random target base stays) |
| `help` | print the command list |
| `quit` | exit |

Validation: coordinates and digits must be in range for the current size; `solve`/`view`
require a current puzzle (`new`/`load` first).

### 1.1 String format

Grid strings use `;` between rows and `,` between block columns within a row. Both
separators are optional when parsing — a flat N² string also works. Digits are `1..=9`,
then hex `a..`; `.` or `0` means empty.

- 9×9: `530,070,000;600,195,000;098,000,060;800,060,003;400,803,001;700,020,006;060,000,280;000,419,005;000,080,079`
- 6×6 `(Bx,By)=(2,3)`: `1.,3.,5.;.4,.6,.2;5.,.2,.4;.3,4.,.1;4.,.1,2.;.1,.3,.5`

The console prints the current grid in this format after every command (`grid: …`).

The size is inferred from the string itself: `;`-separated rows give `N`, the `,`-separated
block columns in a row give `By` (their count), and the block width in cells gives `Bx`
(must satisfy `N = Bx·By`). A flat N² string without separators is read as a square grid
(`Bx = By = √N`) when `N` is a perfect square; otherwise `new <bx> <by>` is required first.

## 2. Target vector semantics

- `target: Vec<f64>` of length `N³`, holding **arbitrary real values** — not restricted to [0, 1].
- The objective is `min −targetᵀ·p`: positive entries pull `p` up, negative entries push it down.
- Base layer: `rand` fills every entry with a uniform random value in **[-1, +1]**.
- Given layer: `set x y n` adds a **singular +10.0 entry** at `idx(x,y,n)` on top of the base,
  so a given dominates the random noise at its variable.
- Internally the UI keeps the random base and the set of givens separate and rebuilds
  `target = base + 10.0 · given_indicator` for every `solve_request`, so `unset`/`clear` are exact.
- Empty cells keep their random base entries; the cell-sum constraint `Σ_n p(x,y,n) = 1` does the work.

## 3. Solver numerics (central path)

- `SolveRequest { size, target, epsilon, start }` with `epsilon` = barrier weight μ > 0 in
  `start` = initial point of the sweep, length N³, clamped into the open interval (0, 1);
  default `1/(bx*by)` in every variable. The solver clamps its result into (0, 1), and the
  console passes the last assignment as `start` of the next solve (warm start).

  ```text
  φ_ε(x) = −targetᵀ·x − ε · Σᵢ ( ln xᵢ + ln(1 − xᵢ) )    s.t.  A·x = b
  ```

- μ-sweep along the central path: μ starts at 1.0 and is multiplied by 0.1 after each
  warm-started Newton solve, until μ ≤ epsilon. The epsilon is UI state (default 1e-6,
  shown in the header), changed only via `eps+` / `eps-` — `solve` never modifies it —
  and never below the numerical floor 1e-10.
- Newton: max 50 iterations per μ, KKT residual tolerance 1e-10.
- The log-barrier enforces `0 < x < 1`, so the bounds need no explicit handling.
- Duality gap of the returned point ≈ `2·N³·ε`.
- Integrality uses a **separate fixed tolerance** `INTEGRAL_TOL = 1e-6` (`|p − round(p)| ≤ tol`
  counts as decided), not ε. Optional branching on a fractional variable (`p = 0` / `p = 1`)
  if an integral solution is required.
- Errors: `Backend(String)` on numerical failure. Note: the pure rule system is always
  feasible (givens enter softly through the target vector), so `Infeasible` /
  `NoIntegralSolution` are only reachable via optional branching. Contradictory givens
  instead show up as fractional probabilities and as flagged row/column conflicts in the
  winner view.

## 4. Output views

View modes are identified by the short strings `winner` / `probs` (stable identifiers for
stored state such as cookies; unknown values fall back to `winner`).

### 4.1 `view winner`

- Per cell the winning digit = `argmax_n p(x, y, n)`, rendered as a grid. Givens get a `*`
  suffix; conflicting entries (winner digit occurring more than once in their row or column)
  get a `c` suffix — both if both apply (`*c`).
  Block boundaries are drawn with `#`: a vertical `#` between block columns, a `#` line between block rows.
- Afterwards every **row** and every **column** is checked for duplicate winners; a row/column
  in which any digit occurs more than once is flagged as erroneous (marked border/asterisk in
  the console output). Blocks are not flagged (rows/columns only, per spec).

### 4.2 `view probs`

- Per cell a `by × bx` mini-grid of percentages: mini-cell `(i, j)` with `i ∈ 0..by`,
  `j ∈ 0..bx` shows the percentage as zero-padded `<nn>%` (`00%`..`99%`); mini-cells within a cell are separated by
  exactly one space (`% `). The console display scales probabilities by `0.99`
  (`DISPLAY_SCALE`, console-render only), so `100%` never occurs and every mini-cell stays
  3 chars. The model/solver/`app` deliver the raw 0..1 values — a different UI (e.g. a
  future HTML frontend) decides itself how to render 0..100% (e.g. color-coded). Within a block, cells are delimited by `:` and
  cell rows by `-` lines; block boundaries stay `#` — so the block boundary reads `%#`
  (vertical `#` between block columns, `#` line between block rows).
- 9×9 → 3×3 mini-grid with digits 1..9 as `3i + j + 1`; 6×6 `(Bx,By) = (2,3)` → 3×2 mini-grid
  with digits 1..6 as `2i + j + 1`.

## 5. Non-goals for v1

- No TUI/GUI/web frontend, no persistence of puzzles, no solver selection via the UI,
  no uniqueness proof of the solution.

## 6. Cookie format (web UI session state)

The web UI state is split across **several cookies** (the 4 KB limit applies per cookie,
so splitting gives more headroom). `sudoku_state` is the manifest: its version tells the
server which other cookies exist and how to decode them. All values are base64url-encoded
(raw `;`, `,`, `"` are not allowed in cookie values).

| cookie | decoded content |
|---|---|
| `sudoku_state` | JSON `{"v":1,"eps":1e-6,"view":"winner"}` — version + small fields |
| `sudoku_givens` | GivenGrid string (§1.1); size `(Bx, By)` inferred via `parse_auto` |
| `sudoku_base` | N³ u8, the random target base in [-1, 1] (`quantize_signed`); target is rebuilt on `solve` as `base + 10.0·givens` |
| `sudoku_start` | optional N³ u8, the last assignment (`quantize_unit`) for the warm start |

- `v` (currently `1`): read `sudoku_state` first, then only the cookies known for that
  version — format changes stay backward compatible and stale cookies are simply ignored.
- Fallbacks per cookie (never reject the whole session): missing/invalid `sudoku_givens` →
  fresh empty session; `sudoku_base` missing/invalid/wrong length → zero base; `eps`
  clamped to `[1e-12, 1.0]`; `view` unknown/missing → `winner`; `sudoku_start`
  missing/invalid/wrong length → `default_start` (1/N in every variable; dequantized values
  are strictly inside (0, 1), so always a valid `start`).
- The cookies are client-controlled: parse defensively, never trust lengths (the existing
  `bx*by <= 64` cap in `Size` applies).
- **No server-side state**: everything lives in the cookies; there is no session store.
  Size (per cookie, base64url of N³ u8): 9×9 ≈ 1 KB, **12×12 (3,4) ≈ 2.3 KB — fits**;
  15×15 (3,5) → N³ = 3375 → ≈ 4.5 KB would overflow the 4 KB per-cookie limit. Therefore
  the web UI only supports sizes up to **12×12** (reject larger `new`/`load` there); the
  console still supports up to `bx*by <= 64`. (The model/solver itself is not the limit —
  15×15 = 3375 variables / 900 rule rows works fine, only the cookie transport is.)
- The cookies contain only puzzle state, no secrets. They are not credentials; still set
  them with `SameSite=Lax`.


