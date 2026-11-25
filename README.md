# sudoku-rs

[![CI](https://github.com/rosenkiesel/sudoku/actions/workflows/ci.yml/badge.svg)](https://github.com/rosenkiesel/sudoku/actions/workflows/ci.yml)

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

The model stores **no givens** — that is the UI's job. The UI builds a target vector
`t` of length `N³` (base: uniform random in [-1,+1] via `rand`, or zeros; each given adds
**+10.0** on top at its variable) and hands `t` plus an **epsilon** to the model. Epsilon is
the barrier weight μ in the objective:

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
- [x] `model` – `Size{bx,by}`, variables `p(x,y,n)`, constraints, `SolveRequest { size, target, epsilon }`
- [x] `solver` – central-path (interior-point) LP with μ-sweep
- [x] `ui` – console REPL + the two output views
- [x] `ui` web (feature `web`, default on) – axum on 127.0.0.1:8080, cookie-only state
- [x] `app` – shared `step()` + generic game loop over `Ui`
- [x] CI – GitHub Actions (fmt, clippy, build, test) on push/PR, badge above

## Console usage

```text
$ cargo run
> new 3 3       # start a 9x9 (or: new 2 3 for a 6x6)
> rand 42       # fill the whole target vector with uniform random in [-1,+1] (seed 42)
> set 0 0 5     # given: target[idx(0,0,5)] += 10.0 on top of the random base
> solve         # central-path solve (default epsilon 1e-6)
> view winner   # argmax per cell + erroneous rows/cols marked
> view probs    # by x bx percentage mini-grid per cell
> load 530,070,000;600,195,000;098,000,060;800,060,003;400,803,001;700,020,006;060,000,280;000,419,005;000,080,079
> quit
```

## Web UI

```bash
cargo run -- web     # serves http://127.0.0.1:8080 (loopback only), then open it in a browser
```

State lives only in cookies (`sudoku_state` manifest + `sudoku_givens`/`sudoku_base`/`sudoku_start`/`sudoku_seeds`,
see [REQUIREMENTS.md](REQUIREMENTS.md) §6) — no server-side session. The web UI supports grids up
to 12×12. The probs view renders the raw 0..100% color-coded. Click a given cell to delete it,
click an empty cell for a digit popup; the rand button uses a fresh time-based seed with a
dropdown of the last ≤ 8 seeds (§7).

**Client-side wasm:** with the wasm build deployed, the browser solves/renders locally and
updates the cookies itself (same Rust cookie codec → byte-identical cookies). Build it with:

```bash
cargo install wasm-pack
wasm-pack build --target web --no-default-features --features wasm --out-dir src/ui/web/static/pkg
```

Without wasm (or without the deployed pkg) the page automatically falls back to
`POST /api/command`, where the server runs the same `app::step` and returns state + cookies.
`cargo run --no-default-features` builds the console-only binary without the web dependencies.

## Development

```bash
cargo fmt --check                 # format check (also in CI)
cargo clippy --all-targets -- -D warnings
cargo test                        # unit + integration tests
cargo run                         # console version
```

CI runs on every push to `main` / `vibe/**` and on PRs — see the badge above.
