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
                    │  LinearSystem, SolveRequest         │
                    └─────────────────────────────────────┘
```

- **model**: pure data + constraint generation. Knows neither UI nor solver.
- **solver**: turns a `LinearSystem` + `SolveRequest` into an `Assignment`.
- **ui**: input/output. Owns the givens (user-entered clues), builds the target vector,
  and renders the two output views. Knows neither model internals nor the solver.
- **app**: generic game loop over `S: Solver` and `U: Ui` — console today, TUI/web later.

## 2. Data model

### 2.1 Parameters

- `b`: block edge length (3 for classic 9×9)
- `N = b * b`: grid edge length
- cells `(x, y)` with `x, y ∈ 0..N` (x = row, y = column)
- digits `n ∈ 1..=N`

### 2.2 Variables

One variable per (cell, digit):

```text
p(x, y, n) ∈ [0, 1]   – probability that cell (x, y) contains digit n
```

Count: `N³` (729 for 9×9). Flat index in a dense vector:

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

4. **Block rule** — block `(bx, by)` covers cells `x ∈ [bx·b, (bx+1)·b)`, `y ∈ [by·b, (by+1)·b)`:

   ```text
   ∀ bx, by, n :  Σ_{(x,y) ∈ Block(bx,by)} p(x, y, n) = 1     → N² rows
   ```

Total `4·N²` rows (324 for 9×9). Bounds: `0.0 <= p(x, y, n) <= 1.0` for every variable.

**No givens in the system.** Givens enter only through the target vector (2.4).

### 2.4 Solve request: target vector + epsilon via central path

```text
SolveRequest {
    target:  Vec<f64>,   // length N³ (729 for 9×9)
    epsilon: f64,        // termination tolerance + integrality threshold
}
```

The UI builds `target` from its givens: for a given `(x, y) = n`,
`target[idx(x,y,n)] = 1.0` and `0.0` for the other digits of that cell; for empty cells a
neutral value (e.g. 0.0 — the constraints do the work).

The solver then computes a solution along the **central path** of the LP

```text
minimize  -targetᵀ·p   subject to   A·p = b,  0 <= p <= 1
```

using a primal-dual interior-point method: at each step take the point of the central path

```text
x(μ) = argmin { -targetᵀ·x - μ·( Σ ln xᵢ + Σ ln(1 - xᵢ) ) : A·x = b }
```

and decrease μ; stop when the duality gap ≤ **epsilon**.

- If every `p` lies within epsilon of `{0, 1}` → integral solution: the digits with `p ≈ 1`.
- Otherwise the fractional values *are* the probabilities (→ probability view); optionally
  branch on a fractional variable (`p = 0` / `p = 1` as extra bounds) to force integrality.

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

- `CentralPathSolver`: interior-point implementation of 2.4.
- Future: `IlpSolver`, `CpSolver` — identical signature.

## 4. UI layer

```rust
pub trait Ui {
    fn render(&mut self, view: &View) -> io::Result<()>;
    fn next_command(&mut self) -> io::Result<Command>;
    fn solve_request(&self, epsilon: f64) -> SolveRequest;   // givens → target vector
}

pub enum Command {
    Set { x: usize, y: usize, n: u8 },
    Unset { x: usize, y: usize },
    Show,
    Solve,
    View(ViewMode),
    Load(String),        // 81-char string, '.' = empty
    Clear,
    Quit,
}

pub enum ViewMode { Winner, Probabilities }
```

- `ConsoleUi`: stdin/stdout REPL with `>` prompt; owns the givens.
- Future: `TuiUi` (e.g. ratatui), `WebUi` — same trait, `app` unchanged.

### 4.1 Output views

`View { givens, assignment: Option<Assignment> }`

1. **Winner** (`ViewMode::Winner`): per cell the winning digit = `argmax_n p(x, y, n)`.
   Afterwards every row and every column is checked for duplicate winners; rows/columns that
   contain a conflict are flagged as erroneous (e.g. marked border / asterisk).
2. **Probabilities** (`ViewMode::Probabilities`): per cell a 3×3 mini-grid; mini-cell `(i, j)`
   shows the percentage of digit `3i + j + 1` from `p(x, y, ·)` (a `b×b` mini-grid for general
   `N = b²`).

## 5. App layer

```rust
pub fn run<S: Solver, U: Ui>(mut ui: U, solver: S, puzzle: &mut Puzzle) -> io::Result<()>
```

Loop: `ui.render(view)` → `ui.next_command()`. On `Solve`: `build_system()` (rules + bounds only)
→ `ui.solve_request(epsilon)` → `solver.solve(...)` → store the `Assignment` in the view.

## 6. Module layout (planned)

```text
Cargo.toml
src/
  main.rs                 // wiring: ConsoleUi + CentralPathSolver -> app::run
  lib.rs
  model/
    mod.rs                // b/N, Cell, Digit, validation
    variables.rs          // idx(x, y, n), num_vars, iteration
    constraints.rs        // build_system() -> LinearSystem   (rules + bounds only)
    request.rs            // SolveRequest { target, epsilon }
  solver/
    mod.rs                // trait Solver, Assignment, SolverError
    central_path.rs       // interior-point / central path
  ui/
    mod.rs                // trait Ui, Command, ViewMode, View
    console.rs            // ConsoleUi: REPL, owns givens, builds target vector
    render.rs             // the two output views
  app.rs                  // run<S: Solver, U: Ui>
tests/
  model.rs                // constraint counts, index mapping, target vector from givens
  integration.rs          // classic puzzle -> integral solution; contradiction -> Infeasible
```

## 7. Test strategy

- **Unit:** `num_vars == N³`, rows `== 4·N²`, index mapping roundtrip, givens → target vector.
- **Integration:** classic 9×9 puzzle with known solution, 4×4 puzzle, empty board (feasible,
  possibly fractional → probability view), contradictory givens → `Infeasible`,
  per-cell probabilities sum to 1.

## 8. Extensions (outlook)

- Other grid sizes (4×4, 16×16) via runtime parameter instead of constants.
- More frontends (TUI via ratatui, web via axum) — new `Ui` impl only.
- Other solvers (ILP, CP-SAT) — new `Solver` impl only.
- Step mode: add constraints incrementally and re-solve (visualize propagation).
