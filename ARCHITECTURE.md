# ARCHITECTURE.md

## 1. Overview

Four layers, strictly separated. `app` only knows traits, never concrete implementations:

```text
┌──────────────┐      ┌──────────────┐      ┌────────────────────┐
│  ui          │      │  app         │      │  solver            │
│  trait Ui    │◄────►│  game loop   │◄────►│  trait Solver      │
│  ConsoleUi   │      │  (generic)   │      │  CentralPathSolver │
└──────────────┘      └──────┬───────┘      └─────────┬──────────┘
                             │                        │
                             ▼                        ▼
                    ┌─────────────────────────────────────┐
                    │  model (pure: no I/O, no solver)    │
                    │  Size, LinearSystem, SolveRequest   │
                    └─────────────────────────────────────┘
```

- **model**: pure data + constraint generation. Knows neither UI nor solver.
- **solver**: turns a `LinearSystem` + `SolveRequest` into an `Assignment`.
- **ui**: input/output. Owns the givens (user-entered clues), builds the target vector,
  and renders the two output views. Knows neither model internals nor the solver.
- **app**: generic game loop over `S: Solver` and `U: Ui` — console today, TUI/web later.

## 2. Data model

### 2.1 Size: `(Bx, By)` instead of a fixed 9×9

The puzzle size is given as **block dimensions**:

```text
Size { bx, by }     // block = bx columns × by rows
N  = bx * by        // grid edge length; digits are 1..=N
grid: N × N cells, (x, y) with x = row, y = column, 0-based
blocks: arranged in a (by × bx) pattern
block (cx, cy):  cx ∈ 0..by, cy ∈ 0..bx
                 columns y ∈ [cx*bx, (cx+1)*bx)
                 rows    x ∈ [cy*by, (cy+1)*by)
```

Examples:

- classic: `Size { bx: 3, by: 3 }` → N = 9, 9×9 grid, 3×3 blocks in a 3×3 pattern, 729 variables
- 6×6 ("6er"): `Size { bx: 2, by: 3 }` → N = 6, 6×6 grid, 2×3 blocks in a 3×2 pattern, 216 variables

### 2.2 Variables

One variable per (cell, digit):

```text
p(x, y, n) ∈ [0, 1]   – probability that cell (x, y) contains digit n
```

Count: `N³`. Flat index in a dense vector:

```text
idx(x, y, n) = (x * N + y) * N + (n - 1)
```

### 2.3 Linear system

```text
LinearSystem {
    num_vars: usize,
    rows:     Vec<(SparseRow, f64)>,   // (coefficients per column, right-hand side b)
    lower:    Vec<f64>,                 // lower bound per variable
    upper:    Vec<f64>,                 // upper bound per variable
}
```

Equations `A·p = b` (all coefficients 1.0):

1. **Cell sum** — each cell carries exactly one digit:

   ```text
   ∀ x, y :  Σ_n p(x, y, n) = 1                              → N² rows
   ```

2. **Row rule** — each digit exactly once per row:

   ```text
   ∀ x, n :  Σ_y p(x, y, n) = 1                              → N² rows
   ```

3. **Column rule**:

   ```text
   ∀ y, n :  Σ_x p(x, y, n) = 1                              → N² rows
   ```

4. **Block rule** — block `(cx, cy)` as defined in 2.1 (contains N cells):

   ```text
   ∀ cx, cy, n :  Σ_{(x,y) ∈ Block(cx,cy)} p(x, y, n) = 1    → N² rows
   ```

Total `4·N²` rows (144 for 6×6, 324 for 9×9). Bounds: `0.0 <= p(x, y, n) <= 1.0` for every
variable. **No givens in the system** — givens enter only through the target vector (2.4).

### 2.4 Solve request: target vector + epsilon (barrier weight) via central path

```text
SolveRequest {
    size:    Size,
    target:  Vec<f64>,   // length N³
    epsilon: f64,        // barrier weight μ > 0
    start:   Vec<f64>,   // length N³, strictly inside (0,1); default 1/(bx*by) per variable
}
```

The UI builds `target` from a random base (uniform in [-1,+1], filled by `rand`) plus its
givens: for a given `(x, y) = n`, `target[idx(x,y,n)] += 10.0` on top of the base (see
REQUIREMENTS.md §2). Without `rand` the base is zero — the constraints do the work.

`start` is the initial point of the central-path sweep. Default: `1/(bx*by)` in every
variable (`default_start`). The solver clamps its result into the open interval (0, 1), so
the previous `Assignment` can be passed as `start` of the next `SolveRequest` — the UI does
exactly that for warm starts.

**Epsilon is the factor with which `log(x)` enters the objective** — the barrier weight μ
on the central path:

```text
φ_ε(x) = −targetᵀ·x − ε · Σᵢ ( ln xᵢ + ln(1 − xᵢ) )    s.t.  A·x = b
```

The log-barrier keeps `0 < x < 1` strictly, so the bounds need no explicit handling in the
solver. The solver follows the central path `C = { x*(μ) : μ > 0 }` with a μ-sweep
(μ₀ = 1.0, then μ ← μ/10) and warm-started Newton steps, down to `max(ε, 1e-10)` (numerical
floor); the returned point `x*(ε)` has estimated duality gap ≈ `2·N³·ε`.

- ε small (e.g. 1e-6) → near the LP optimum: peaked distribution, ≈ 0/1 where decided → winner view.
- ε large (e.g. 1.0) → near the analytic center of the feasible polytope: flat probabilities → probability view.
- Integrality classification / optional branching (`p = 0` / `p = 1` as extra bounds) uses a
  **separate fixed tolerance**, not ε.

## 3. Solver layer

```rust
pub trait Solver {
    fn solve(&self, system: &LinearSystem, request: &SolveRequest) -> Result<Assignment, SolverError>;
}

pub struct Assignment { pub values: Vec<f64> }   // length = num_vars

pub enum SolverError {
    Infeasible,            // contradictory input
    NoIntegralSolution,    // branching found no 0/1 solution
    Backend(String),       // error from the numerical backend
}
```

- `CentralPathSolver`: primal-dual interior point with μ-sweep, implementation of 2.4.
  The rule matrix `A` has rank `4·N² − 3` (the four rule families have equal total sums), so
  the solver drops the three linearly dependent rows before forming the Newton Schur system
  and adds a tiny Tikhonov term (`reg`) for numerical safety.
- Future: `IlpSolver`, `CpSolver` — identical signature.

## 4. UI layer

```rust
pub trait Ui {
    fn next_command(&mut self) -> io::Result<Command>;
    fn apply(&mut self, command: &Command) -> io::Result<()>;
    fn solve_request(&self) -> Option<SolveRequest>;   // givens + base + current epsilon → request
    fn set_assignment(&mut self, assignment: Option<Assignment>);
    fn note(&mut self, message: &str);
    fn render(&mut self) -> io::Result<()>;
}

pub enum Command {
    New { bx: usize, by: usize },   // start a puzzle of size (Bx, By)
    Rand { seed: Option<u64> },     // fill the target base with uniform random in [-1,+1]
    Set { x: usize, y: usize, n: u8 },
    Unset { x: usize, y: usize },
    Show,
    Help,
    Solve,
    EpsPlus,
    EpsMinus,
    View(ViewMode),
    Load(String),        // grid string: rows ';', block columns ',' (REQUIREMENTS §1.1)
    Clear,
    Quit,
}

pub enum ViewMode { Winner, Probabilities }
```

- `ConsoleUi`: stdin/stdout REPL with `>` prompt; owns the current `Size` and the givens.
- Future: `TuiUi` (e.g. ratatui), `WebUi` — same trait, `app` unchanged.

### 4.1 Output views

The render functions receive `size`, the givens and the optional `Assignment`.

1. **Winner** (`ViewMode::Winner`): per cell the winning digit = `argmax_n p(x, y, n)`.
   Afterwards every row and every column is checked for duplicate winners; rows/columns that
   contain a conflict are flagged as erroneous. Grid rendering: block boundaries use `#`
   (vertical `#` between block columns, `#` line between block rows); givens get a `*` suffix
   and conflicting entries a `c` suffix in the grid.
2. **Probabilities** (`ViewMode::Probabilities`): per cell a `by × bx` mini-grid; mini-cell
   `(i, j)` with `i ∈ 0..by`, `j ∈ 0..bx` shows the percentage of digit `i·bx + j + 1`
   (3×3 for 9×9, 3×2 for 6×6). Within a block, cells are delimited by `:` and cell rows by
   `-` lines; block boundaries stay `#`.

## 5. App layer

```rust
pub fn run<S: Solver, U: Ui>(mut ui: U, solver: &S) -> io::Result<()>
```

Loop: `ui.render()` → `ui.next_command()`. On `Solve`: `build_system(size)` (rules + bounds
only) → `ui.solve_request()` (the UI owns the current epsilon, changed via `EpsPlus`/`EpsMinus`)
→ `solver.solve(...)` → `ui.set_assignment(...)`.
The assignment carries the raw 0..1 values; display scaling (console: ×0.99 so `100%`
never occurs) is a rendering detail of each UI, not of the model.

## 6. Module layout

```text
Cargo.toml
src/
  main.rs                 // wiring: ConsoleUi + CentralPathSolver -> app::run
  lib.rs
  model/
    mod.rs                // Size { bx, by }, N = bx*by, Cell, Digit, validation
    grid_indices.rs       // idx(x, y, n), num_vars, iteration
    given_grid.rs         // GivenGrid: givens + dimensions() + parse/parse_auto/format (';' rows, ',' block cols; size inferred from separators)
    game_rules.rs         // build_system(size) -> LinearSystem   (rules + bounds only)
    request.rs            // SolveRequest { size, target, epsilon, start }, default_start
  solver/
    mod.rs                // trait Solver, Assignment, SolverError
    central_path.rs       // interior point, μ-sweep along the central path
  ui/
    mod.rs                // generic: trait Ui, Command, ViewMode, DEFAULT_EPSILON
    console/
      mod.rs              // ConsoleUi (console-specific): REPL, owns size + givens + epsilon
      render.rs           // the two output views (console-specific rendering)
  app.rs                  // run<S: Solver, U: Ui>
tests/
  model.rs                // constraint counts for (3,3) and (2,3), index mapping, target vector
  integration.rs          // 9×9 and 6×6 puzzles solved exactly; contradictory givens stay
                          // feasible (soft target) but are flagged as a row conflict
```

## 7. Test strategy

- **Unit:** for `Size{3,3}` and `Size{2,3}`: `num_vars == N³`, rows `== 4·N²`, block coverage
  (every cell in exactly one block), index mapping roundtrip, givens → target vector.
- **Integration:** classic 9×9 puzzle with known solution, a 6×6 puzzle, empty board
  (feasible, possibly fractional → probability view), contradictory givens → `Infeasible`,
  per-cell probabilities sum to 1.

## 8. Extensions (outlook)

### Web UI (discussion, not implemented)

- New module `src/ui/web/` (e.g. axum): `GET /` renders HTML (winner table, probs heatmap
  color-coded over the raw 0..1 values — no `DISPLAY_SCALE`), `POST /command` applies one
  command per request; `app` exposes a per-request `step(ui, solver, command)` instead of
  only the blocking REPL loop.
- Per-session state, compact enough for a 4 KB cookie: givens as the grid string
  (~100 B, size inferred via `parse_auto`), target base as a quantized u8 vector
  (N³ B, `model::quantize::quantize_signed`), epsilon as short text, view mode as short
  string (`"winner"`/`"probs"`, unknown values fall back to the default), last assignment
  quantized to u8 (N³ B, `quantize_unit`/`dequantize_unit` — dequantized values are
  strictly inside (0,1), so always a valid warm-start `start`). Total ≈ 2 KB for 9×9,
  split across several cookies (`sudoku_state` manifest with version + `sudoku_givens` /
  `sudoku_base` / `sudoku_start`); the 4 KB limit applies per cookie. Cookie format:
  REQUIREMENTS.md §6. Alternative for large grids: session-id cookie + server-side store
  with full f64 state.


- More frontends (TUI via ratatui, web via axum) — new `Ui` impl only.
- Other solvers (ILP, CP-SAT) — new `Solver` impl only.
- Step mode: add constraints incrementally and re-solve (visualize propagation).
- Rectangular grids (Nx ≠ Ny) if ever needed — the block rule generalizes directly.
