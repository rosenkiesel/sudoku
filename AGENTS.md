# AGENTS.md — Top-Down V-Model & Vibe-Governance

Project-wide rules for anyone (human or agent) working in this repository.

This project uses a lightweight, ASPICE-inspired **Top-Down V-Model** to combine rapid AI prototyping ("vibe coding") with a stable, human-understood system core.

---

## 0. Language Policy (Strict English)

* **Repository & Artifacts:** All files in this repository—including requirements, architecture docs, code identifiers, comments, commit messages, and tests—**must be written in English**.
* **Prompt Independence:** Even if the user, tester, or developer communicates with you in another language (e.g., German, French, Spanish), always translate the intent and write all repository artifacts strictly in English.

---

## 1. Naming Scheme (Ownership & Maturity)

The identifier of every artifact (requirement, module, class, function, test) encodes whether it is part of the human mental model or volatile AI-generated prototype code:

| V-Model Layer (ASPICE) | AI Prototype (Volatile) | Human Core (Reviewed & Stable) |
| :--- | :--- | :--- |
| **SWE.1 Requirements** | `REQ-VIBE-<NAME>` / `REQ-VIBE-001` | `REQ-001` |
| **SWE.2 Architecture** | `MOD-VIBE-<NAME>` / `vibe_<module>/` | `MOD-<NAME>` / `<module>/` |
| **SWE.3 Functions / Variables** | `vibe_<name>()` | `<name>()` |
| **SWE.3 Classes / Types** | `Vibe<Name>` | `<Name>` |
| **SWE.4–6 Tests** | `test_vibe_<name>()` | `test_<name>()` |

### Golden Naming Rules:
1. **Everything New Starts as `VIBE`:** Every requirement, module, class, function, or test that you (the AI) create **must** include `VIBE` or `vibe_` in its name.
2. **No Self-Promotion:** You must never remove the `vibe_` / `VIBE` prefix on your own. Promoting an element (`vibe_foo` → `foo`, `REQ-VIBE-001` → `REQ-001`) is an exclusive human review act.
3. **Full Freedom in `VIBE` Scope:** You may freely rewrite, refactor, merge, or delete any element marked with `vibe_` / `VIBE` at any time to achieve a working prototype.
4. **Core Protection (Non-VIBE):** Elements **without** a `vibe_` / `VIBE` marker belong to the verified human mental model. You **must not** silently alter their specification, public signature, or defined behavior.

---

## 2. Top-Down Cascade (V-Model Flow)

The driving direction for all structural and behavioral changes flows strictly from top to bottom:

```text
[1. SWE.1 Requirements]  (docs/01_requirements/)
          │
          ▼
[2. SWE.2 Architecture]   (docs/02_architecture/)
          │
          ▼
[3. SWE.6 Verification]  (tests/verification/)
          │
          ▼
[4. SWE.3 Code / Vibe]   (src/)
```

In this repository the layers map to: SWE.1 → `REQUIREMENTS.md`, SWE.2 → `ARCHITECTURE.md` (+ `README.md` overview), SWE.6 → `tests/`, SWE.3 → `src/`. Work happens on `vibe/…` branches (already VIBE-marked); no PR unless explicitly requested.

---

## 3. Architecture principles

- Strict layering: `model` (pure data + linear system, no I/O, no solver) → `solver` (`trait Solver`) and `ui` (`trait Ui`) → `app` (generic game loop over both traits).
- The console is one frontend; everything user-facing goes through `trait Ui`. `model` and `solver` must never depend on a concrete UI, and the UI must never depend on solver internals. The web UI is a second `ui` implementation (`ui/web`) plus an optional wasm build (`src/wasm.rs`).
- Same for the solver: central-path LP today; other backends (ILP, CP) must slot into `trait Solver` without touching the rest.

## 4. Data model

- Grid size is given as **block dimensions** `(Bx, By)`: `N × N` grid with `N = Bx·By`, digits `1..=N`, blocks of `Bx` columns × `By` rows in a `By × Bx` pattern. Examples: classic 9×9 = `(3, 3)`; 6×6 ("6er") = `(2, 3)` → 2×3 blocks in a 3×2 pattern on a 6×6 field.
- Probability vector: `p(x, y, n)` = probability that cell `(x, y)` (x = row, y = column, 0-based) contains digit `n`. One variable per (cell, digit): `N³` variables (729 for 9×9, 216 for 6×6).
- Sudoku rules as **linear equations** over `p` (see ARCHITECTURE.md), plus the bounds `0.0 <= p(x, y, n) <= 1.0`.
- **No total givens in the model.** Givens are a UI concern. The model receives a **target vector** `t` (length `N³`), an **epsilon**, and a **start state**, and determines a solution on the **central path** of the LP. Epsilon is the **barrier weight μ** — the factor with which `log(x)` enters the objective:

  ```text
  φ_ε(x) = -tᵀ·x - ε · Σᵢ ( ln xᵢ + ln(1 - xᵢ) )    s.t.  A·x = b
  ```

  The log-barrier keeps `0 < x < 1` strictly, so the bounds need no explicit handling in the solver. The solver follows the central path with a μ-sweep (warm-started Newton; the previous assignment is reused as `start`); the returned point has duality gap ≈ `2·N³·ε`. Small ε → near the LP optimum (peaked, ≈ 0/1 → winner view); large ε → near the analytic center (flat → probability view). Integrality classification uses a separate fixed tolerance, **not** ε.

## 5. Output

The UI renders the solution in one of two views:

1. **Winner view** — per cell the winning digit (argmax of `p`), plus a marking of which rows or columns contain errors (conflicts) after that assignment.
2. **Probability view** — per cell a `By × Bx` mini-grid with the percentage probability of each digit (mini-cell `(i, j)` shows digit `i·Bx + j + 1`; 3×3 for 9×9, 3×2 for 6×6). The console scales the display by 0.99 (so `100%` never occurs); the web UI renders the raw 0..100% color-coded.

## 6. Workflow

- Docs-first: `README.md` (overview), `ARCHITECTURE.md` (layering + linear model) and `REQUIREMENTS.md` (detailed interface, cookie format, and numerical specs) describe the design before code lands in this repo.
- Verify with `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and for the web/wasm parts `cargo check --target wasm32-unknown-unknown --no-default-features --features wasm` before pushing. CI runs all of these.
- Work happens on `vibe/…` branches; no PR unless explicitly requested.
