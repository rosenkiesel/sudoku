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

- Probability vector: `p(x, y, n)` = probability that cell `(x, y)` contains digit `n`. One variable per (cell, digit): N³ variables (729 for 9×9).
- Sudoku rules as **linear equations** over `p` (see ARCHITECTURE.md), plus the bounds `0.0 <= p(x, y, n) <= 1.0`.
- **No total givens in the model.** Givens are a UI concern. The model receives a **target vector** `t` (9×9×9 = 729 entries for classic Sudoku) and an **epsilon**, and determines a solution along the **central path** of the LP:

  ```text
  minimize  -tᵀ·p   subject to   A·p = b,  0 <= p <= 1
  ```

  solved with a primal-dual interior-point method, decreasing μ until the duality gap ≤ epsilon. Epsilon doubles as the integrality threshold (p within epsilon of {0, 1} counts as decided).

## Output

The UI renders the solution in one of two views:

1. **Winner view** — per cell the winning digit (argmax of `p`), plus a marking of which rows or columns contain errors (conflicts) after that assignment.
2. **Probability view** — per cell a 3×3 mini-grid holding the percentage probabilities of the digits 1..=9.

## Workflow

- Docs-first: `README.md` and `ARCHITECTURE.md` describe the design before code lands in this repo.
- Verify with `cargo test` / `cargo run` before pushing.
- Work happens on `vibe/…` branches; no PR unless explicitly requested.
