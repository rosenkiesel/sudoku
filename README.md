# sudoku-rs

Probabilistic Sudoku in Rust: cells are modeled as a probability vector, the rules are linear
equations, and the solution is computed along the central path of the LP. Console UI first,
abstracted behind traits so any other frontend can replace it later.

Repo-wide rules (language, layering, model contract) live in [AGENTS.md](AGENTS.md).

## Size: `(Bx, By)` — not fixed to 9×9

The puzzle size is the **block size**: `Size { bx, by }` means a `N × N` grid with
`N = Bx·By`, digits `1..=N`, and `Bx × By` blocks arranged in a `By × Bx` pattern.

- classic: `(3, 3)` → 9×9, 3×3 blocks, 729 variables
- 6×6: `(2, 3)` → 2×3 blocks in a 3×2 pattern on a 6×6 field, 216 variables

## Data model

```text
p(x, y, n) = probability that cell (x, y) contains digit n
            (x = row, y = column, 0-based; n = 1..=N)
```

Rules as linear equations, e.g.:

```text
∀ x, n :  1 == Σ_y p(x, y, n)          (each row contains each digit exactly once)
```

plus the bounds `0.0 <= p(x, y, n) <= 1.0`.

The model stores **no givens** — that is the UI's job. The UI maps givens to a target vector
`t ∈ [0,1]^N³` (1.0 for a given digit, 0.0 for the other digits of that cell) and hands `t`
plus an **epsilon** to the model. Epsilon is the barrier weight μ in the objective:

```text
φ_ε(x) = −tᵀ·x − ε · Σᵢ ( ln xᵢ + ln(1 − xᵢ) )    s.t.  A·x = b
```

solved along the central path (interior point, μ-sweep); duality gap ≈ `2·N³·ε`. Small ε →
peaked, near 0/1; large ε → flat probabilities near the analytic center.

## Output

- per cell the **winner** (argmax of `p`) plus flagged **rows/columns that contain conflicts**, or
- per cell a `By × Bx` mini-grid with the percentage probability of each digit (3×3 for 9×9, 3×2 for 6×6).

Architecture details in [ARCHITECTURE.md](ARCHITECTURE.md); detailed interface and numerical
specs in [REQUIREMENTS.md](REQUIREMENTS.md).

## Status

- [x] `README.md`, `ARCHITECTURE.md`, `REQUIREMENTS.md`, `AGENTS.md`
- [ ] `model` – `Size{bx,by}`, variables `p(x,y,n)`, constraints, `SolveRequest { size, target, epsilon }`
- [ ] `solver` – central-path (interior-point) LP with μ-sweep
- [ ] `ui` – console REPL + the two output views
- [ ] `app` – generic game loop over `Solver` and `Ui`

## Planned console usage

```text
$ cargo run
> new 3 3       # start a 9x9 (or: new 2 3 for a 6x6)
> rand 42       # fill the whole target vector with uniform random in [-1,+1] (seed 42)
> set 0 0 5     # given: target[idx(0,0,5)] += 10.0 on top of the random base
> solve         # central-path solve (default epsilon 1e-6)
> view winner   # argmax per cell + erroneous rows/cols marked
> view probs    # by x bx percentage mini-grid per cell
> load 530070600600195342098000060800060003400803001700020006060000280000419005000080079
> quit
```

## Development

```bash
cargo test    # unit + integration tests
cargo run     # console version
```
