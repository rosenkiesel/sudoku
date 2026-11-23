# AGENTS.md

Project-wide rules for anyone (human or agent) working in this repository.

## Language

- The repository is **English-only**: code, identifiers, comments, documentation, commit messages, issues, PRs — always English.
- This holds regardless of the language used in conversations with an agent. Discussions may be in German or any other language; files in this repo are not.

## Architecture principles

- Strict layering: `model` (pure data + linear system, no I/O, no solver) → `solver` (`trait Solver`) and `ui` (`trait Ui`) → `app` (generic game loop over both traits).
- The console is the first frontend, but everything user-facing goes through `trait Ui`. `model` and `solver` must never depend on a concrete UI, and the UI must never depend on solver internals.
- Same for the solver: central-path LP today; other backends (ILP, CP) must slot into `trait Solver` without touching the rest.

## Data model

- Grid size is given as **block dimensions** `(Bx, By)`: `N × N` grid with `N = Bx·By`, digits `1..=N`, blocks of `Bx` columns × `By` rows in a `By × Bx` pattern. Examples: classic 9×9 = `(3, 3)`; 6×6 ("6er") = `(2, 3)` → 2×3 blocks in a 3×2 pattern on a 6×6 field.
- Probability vector: `p(x, y, n)` = probability that cell `(x, y)` (x = row, y = column, 0-based) contains digit `n`. One variable per (cell, digit): `N³` variables (729 for 9×9, 216 for 6×6).
- Sudoku rules as **linear equations** over `p` (see ARCHITECTURE.md), plus the bounds `0.0 <= p(x, y, n) <= 1.0`.
- **No total givens in the model.** Givens are a UI concern. The model receives a **target vector** `t` (length `N³`) and an **epsilon**, and determines a solution on the **central path** of the LP. Epsilon is the **barrier weight μ** — the factor with which `log(x)` enters the objective:

  ```text
  φ_ε(x) = -tᵀ·x - ε · Σᵢ ( ln xᵢ + ln(1 - xᵢ) )    s.t.  A·x = b
  ```

  The log-barrier keeps `0 < x < 1` strictly, so the bounds need no explicit handling in the solver. The solver follows the central path with a μ-sweep (warm-started Newton); the returned point has duality gap ≈ `2·N³·ε`. Small ε → near the LP optimum (peaked, ≈ 0/1 → winner view); large ε → near the analytic center (flat → probability view). Integrality classification uses a separate fixed tolerance, **not** ε.

## Output

The UI renders the solution in one of two views:

1. **Winner view** — per cell the winning digit (argmax of `p`), plus a marking of which rows or columns contain errors (conflicts) after that assignment.
2. **Probability view** — per cell a `By × Bx` mini-grid with the percentage probability of each digit (mini-cell `(i, j)` shows digit `i·Bx + j + 1`; 3×3 for 9×9, 3×2 for 6×6).

## Workflow

- Docs-first: `README.md` (overview), `ARCHITECTURE.md` (layering + linear model) and `REQUIREMENTS.md` (detailed interface and numerical specs) describe the design before code lands in this repo.
- Verify with `cargo test` / `cargo run` before pushing.
- Work happens on `vibe/…` branches; no PR unless explicitly requested.
