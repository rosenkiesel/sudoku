# sudoku-rs

Probabilistic Sudoku in Rust: cells are modeled as a probability vector, the rules are linear
equations, and the solution is computed along the central path of the LP. Console UI first,
abstracted behind traits so any other frontend can replace it later.

Repo-wide rules (language, layering, model contract) live in [AGENTS.md](AGENTS.md).

## Data model

```text
p(x, y, n) = probability that cell (x, y) contains digit n
            (x = row, y = column, 0-based; n = 1..=N; N = b²)
```

Rules as linear equations, e.g.:

```text
∀ x, n :  1 == Σ_y p(x, y, n)          (each row contains each digit exactly once)
```

plus the bounds:

```text
0.0 <= p(x, y, n) <= 1.0
```

The model stores **no givens** — that is the UI's job. The UI maps givens to a target vector
`t ∈ [0,1]^(9·9·9)` (1.0 for a given digit, 0.0 for the other digits of that cell) and hands
`t` plus an **epsilon** to the model, which solves

```text
minimize  -tᵀ·p   subject to   A·p = b,  0 <= p <= 1
```

along the central path (interior-point), stopping when the duality gap ≤ epsilon.

## Output

- per cell the **winner** (argmax of `p`) plus flagged **rows/columns that contain conflicts**, or
- per cell a **3×3 mini-grid** with the percentage probability of each digit 1..=9.

Details in [ARCHITECTURE.md](ARCHITECTURE.md).

## Status

- [x] `README.md`, `ARCHITECTURE.md`, `AGENTS.md`
- [ ] `model` – variables `p(x,y,n)`, constraints, `SolveRequest { target, epsilon }`
- [ ] `solver` – central-path (interior-point) LP
- [ ] `ui` – console REPL + the two output views
- [ ] `app` – generic game loop over `Solver` and `Ui`

## Planned console usage

```text
$ cargo run
> set 0 0 5     # row 0, col 0 = 5 (given; UI turns it into the target vector)
> solve         # central-path solve
> view winner   # argmax per cell + erroneous rows/cols marked
> view probs    # 3x3 percentage grid per cell
> load 530070600600195342098000060800060003400803001700020006060000280000419005000080079
> quit
```

## Development

```bash
cargo test    # unit + integration tests
cargo run     # console version
```
