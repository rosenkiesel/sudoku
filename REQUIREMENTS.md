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
| `rand [seed]` | fill the **entire target vector** with uniform random values in **[-1, +1]**; optional seed for reproducibility; resets givens |
| `set <x> <y> <n>` | add a given: `target[idx(x,y,n)] += 10.0`, on top of the current value (e.g. the random base). At most one given per cell; setting an occupied cell replaces the old given |
| `unset <x> <y>` | remove the given at cell `(x, y)` |
| `solve [eps]` | build `LinearSystem` + `SolveRequest` and run the central-path solver; default `eps = 1e-6` |
| `view winner` | winner view (§4.1) |
| `view probs` | probability view (§4.2) |
| `show` | current size, givens and a summary of the target vector |
| `load <string>` | N² characters, digit chars for 1..=N (hex digits beyond 9), `.` or `0` = empty; each digit becomes a given |
| `clear` | remove all givens (the random target base stays) |
| `quit` | exit |

Validation: coordinates and digits must be in range for the current size; `solve`/`view`
require a current puzzle (`new`/`load` first).

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

- `SolveRequest { size, target, epsilon }` with `epsilon` = barrier weight μ > 0 in

  ```text
  φ_ε(x) = −targetᵀ·x − ε · Σᵢ ( ln xᵢ + ln(1 − xᵢ) )    s.t.  A·x = b
  ```

- μ-sweep along the central path: μ starts at 1.0 and is multiplied by 0.1 after each
  warm-started Newton solve, until μ ≤ epsilon (the `solve [eps]` argument, default 1e-6),
  but never below the numerical floor 1e-10.
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

### 4.1 `view winner`

- Per cell the winning digit = `argmax_n p(x, y, n)`, rendered as a grid; givens are marked (e.g. `*` suffix).
- Afterwards every **row** and every **column** is checked for duplicate winners; a row/column
  in which any digit occurs more than once is flagged as erroneous (marked border/asterisk in
  the console output). Blocks are not flagged (rows/columns only, per spec).

### 4.2 `view probs`

- Per cell a `by × bx` mini-grid of percentages: mini-cell `(i, j)` with `i ∈ 0..by`,
  `j ∈ 0..bx` shows `100 · p(x, y, i·bx + j + 1)`.
- 9×9 → 3×3 mini-grid with digits 1..9 as `3i + j + 1`; 6×6 `(Bx,By) = (2,3)` → 3×2 mini-grid
  with digits 1..6 as `2i + j + 1`.

## 5. Non-goals for v1

- No TUI/GUI/web frontend, no persistence of puzzles, no solver selection via the UI,
  no uniqueness proof of the solution.
