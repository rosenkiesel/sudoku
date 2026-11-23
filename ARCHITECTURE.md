# ARCHITECTURE.md

## 1. Überblick

Vier Schichten, strikt getrennt. `app` kennt nur Traits, nie konkrete Implementierungen:

```text
┌──────────────┐      ┌──────────────┐      ┌──────────────┐
│  ui          │      │  app         │      │  solver      │
│  trait Ui    │◄────►│  Game-Loop   │◄────►│  trait Solver│
│  ConsoleUi   │      │  (generisch) │      │  LpSolver    │
└──────────────┘      └──────┬───────┘      └──────┬───────┘
                             │                     │
                             ▼                     ▼
                    ┌───────────────────────────────────┐
                    │  model (rein: kein I/O, kein LP)  │
                    │  Puzzle, p(x,y,n), LinearSystem   │
                    └───────────────────────────────────┘
```

- **model**: reine Daten + Constraint-Generierung. Kennt weder Konsole noch LP-Solver.
- **solver**: löst ein `LinearSystem` zu einer Belegung aller Variablen.
- **ui**: Ein-/Ausgabe. Kennt weder Modell-Details noch den Solver.
- **app**: Game-Loop, generisch über `S: Solver` und `U: Ui` – Konsole heute, TUI/Web morgen.

## 2. Datenmodell

### 2.1 Parameter

- `b`: Block-Kantenlänge (3 beim klassischen 9×9)
- `N = b * b`: Gitter-Kantenlänge
- Zellen `(x, y)` mit `x, y ∈ 0..N` (x = Zeile, y = Spalte)
- Ziffern `n ∈ 1..=N`

### 2.2 Variablen

Eine Variable pro (Zelle, Ziffer):

```text
p(x, y, n) ∈ [0, 1]   – Wahrscheinlichkeit, dass Zelle (x, y) die Ziffer n enthält
```

Anzahl: `N³` (729 bei 9×9). Flachindex in einem dichten Vektor:

```text
idx(x, y, n) = (x * N + y) * N + (n - 1)
```

### 2.3 Lineares System

```text
LinearSystem {
    num_vars: usize,
    rows:     Vec<(SparseRow, f64)>,   // (Koeffizienten pro Spalte, rechte Seite b)
    lower:    Vec<f64>,                 // untere Schranke pro Variable
    upper:    Vec<f64>,                 // obere Schranke pro Variable
}
```

#### Gleichungen `A·p = b` (alle Koeffizienten 1.0)

1. **Zellensumme** – jede Zelle trägt genau eine Ziffer:

   ```text
   ∀ x, y :  Σ_n p(x, y, n) = 1                → N² Zeilen
   ```

2. **Zeilen-Regel** – jede Ziffer genau einmal pro Zeile:

   ```text
   ∀ x, n :  Σ_y p(x, y, n) = 1                → N² Zeilen
   ```

3. **Spalten-Regel** – jede Ziffer genau einmal pro Spalte:

   ```text
   ∀ y, n :  Σ_x p(x, y, n) = 1                → N² Zeilen
   ```

4. **Block-Regel** – Block `(bx, by)` umfasst die Zellen `x ∈ [bx·b, (bx+1)·b)`, `y ∈ [by·b, (by+1)·b)`:

   ```text
   ∀ bx, by, n :  Σ_{(x,y) ∈ Block(bx,by)} p(x, y, n) = 1   → N² Zeilen
   ```

Insgesamt `4·N²` Gleichungen (324 bei 9×9).

#### Givens (vorgegebene Ziffern)

Nicht als eigene Zeile, sondern als **fixierte Bounds**:

- Given `(x, y) = n`: `lower[idx(x,y,n)] = upper[idx(x,y,n)] = 1.0`
- und `lower = upper = 0.0` für alle `m ≠ n` in derselben Zelle.

#### Bounds

Global für alle Variablen:

```text
0.0 <= p(x, y, n) <= 1.0
```

### 2.4 Von der LP zur Lösung

- Es handelt sich um ein reines Feasibility-Problem: Zielfunktion konstant 0, das LP liefert eine zulässige Belegung.
- **Integralität:** Die Constraint-Matrix ist nicht total unimodular (es existieren fraktionale Latin Squares),
  daher *kann* das LP-Optimum bei schwach eingeschränkten Rätseln fraktional sein. Strategie:
  1. LP lösen; liegen alle Werte innerhalb ε ∈ {0, 1} → 0/1-Lösung gefunden.
  2. Sonst **Branch & Bound**: eine fraktionale Variable wählen und verzweigen mit `p = 0` bzw. `p = 1`
     (als zusätzliche Bounds), rekursiv weiterlösen.
  Mit hinreichend vielen Givens ist Schritt 1 typischerweise direkt integral.
- Die gefundene 0/1-Belegung wird gegen die Regeln verifiziert und als gelöste Zellen übernommen.
  Eine Eindeutigkeitsprüfung (jede Zelle auf alternativen Wert testen) ist als optionale Erweiterung vorgesehen.

## 3. Solver-Schicht

```rust
pub trait Solver {
    fn solve(&self, system: &LinearSystem) -> Result<Assignment, SolverError>;
}

pub struct Assignment { pub values: Vec<f64> }   // Länge = num_vars

pub enum SolverError {
    Infeasible,            // widersprüchliche Givens
    NoIntegralSolution,    // Branch & Bound ohne 0/1-Lösung
    Backend(String),       // Fehler des LP-Backends
}
```

- `LpSolver`: LP-Backend (z. B. HiGHS über `good_lp`), implementiert Branch & Bound aus 2.4.
- Zukünftig: `IlpSolver`, `CpSolver` – exakt dieselbe Signatur.

## 4. UI-Schicht

```rust
pub trait Ui {
    fn render(&mut self, view: &View) -> io::Result<()>;
    fn next_command(&mut self) -> io::Result<Command>;
}

pub enum Command {
    Set { x: usize, y: usize, n: u8 },
    Unset { x: usize, y: usize },
    Show,
    Solve,
    Prob { x: usize, y: usize },
    Load(String),        // 81-stelliger String, '.' = leer
    Clear,
    Quit,
}
```

- `ConsoleUi`: REPL über stdin/stdout mit Prompt `>`.
- Zukünftig: `TuiUi` (z. B. ratatui), `WebUi` – gleiche Signatur, `app` bleibt unverändert.
- `View`: Snapshot des Zustands (Givens, letzte Lösung, Wahrscheinlichkeiten), den das UI nur rendert.

## 5. App-Schicht

```rust
pub fn run<S: Solver, U: Ui>(mut ui: U, solver: S, puzzle: &mut Puzzle) -> io::Result<()>
```

Ablauf: `loop { ui.render(&view); match ui.next_command()? { ... } }`

- `Solve` baut über `model::constraints::build_system(puzzle)` das `LinearSystem`, ruft den Solver auf
  und übernimmt die 0/1-Werte als gelöste Zellen in die Ansicht.
- `Prob x y` zeigt die Verteilung `p(x, y, :)` aus der letzten Assignment an.

## 6. Modul-Layout (geplant)

```text
Cargo.toml
src/
  main.rs                 // Wiring: ConsoleUi + LpSolver -> app::run
  lib.rs
  model/
    mod.rs                // Puzzle, Cell, Digit, b/N, Validierung der Givens
    variables.rs          // idx(x, y, n), num_vars, Iteration über Variablen
    constraints.rs        // build_system(&Puzzle) -> LinearSystem
  solver/
    mod.rs                // trait Solver, Assignment, SolverError
    lp.rs                 // LpSolver: LP + Branch & Bound
  ui/
    mod.rs                // trait Ui, Command, View
    console.rs            // ConsoleUi
  app.rs                  // run<S: Solver, U: Ui>
tests/
  model.rs                // Constraint-Anzahlen, Index-Mapping, Givens-Bounds
  integration.rs          // klassisches Rätsel -> Lösung; leeres Brett; Widerspruch -> Infeasible
```

## 7. Teststrategie

- **Unit:** `num_vars == N³`, Zeilenzahl `== 4·N²`, Index-Mapping (Hin- und Rückweg),
  Givens setzen fixiert die Bounds korrekt, ungültige Givens werden abgelehnt.
- **Integration:** klassisches 9×9-Rätsel mit bekannter Lösung, 4×4-Rätsel,
  leeres Brett (feasible, ggf. fraktional → Branching-Pfad), widersprüchliche Givens → `Infeasible`.

## 8. Erweiterungen (Ausblick)

- Andere Gittergrößen (4×4, 16×16) über Laufzeit-Parameter statt Konstanten.
- Weiteres UI (TUI via ratatui, Web via axum) – nur neue `Ui`-Implementierung.
- Anderer Solver (ILP, CP-SAT) – nur neue `Solver`-Implementierung.
- Wahrscheinlichkeits-Anzeige pro Zelle direkt aus der LP-Lösung (im Modell bereits vorgesehen).
- Schrittmodus: Constraints einzeln hinzufügen und das LP jeweils neu lösen (Visualisierung des Propagierens).
