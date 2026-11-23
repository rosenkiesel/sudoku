# sudoku-rs

Sudoku in Rust, modelliert als **Wahrscheinlichkeitsvektor** und gelöst per **linearer Programmierung (LP)**.

Die Konsolen-Oberfläche ist über ein Trait abstrahiert, sodass später z. B. ein TUI oder ein Web-Frontend
angebunden werden kann, ohne Modell oder Solver zu ändern. Details siehe [ARCHITECTURE.md](ARCHITECTURE.md).

## Datenmodell

Für jede Zelle `(x, y)` (x = Zeile, y = Spalte, 0-basiert) und jede Ziffer `n` (1..=N) gibt es eine Variable:

```text
p(x, y, n) = Wahrscheinlichkeit, dass in Zelle (x, y) die Ziffer n eingetragen ist
```

Die Sudoku-Regeln werden als **lineare Gleichungen** über diese Variablen formuliert, z. B.:

```text
∀ x, n :  1 == Σ_y p(x, y, n)        (jede Zeile enthält jede Ziffer genau einmal)
```

Dazu die Bounds:

```text
0.0 <= p(x, y, n) <= 1.0
```

Alle Gleichungen (Zelle, Zeile, Spalte, Block, Givens) sind in [ARCHITECTURE.md](ARCHITECTURE.md) aufgeführt.

## Status

Dokumentation zuerst – die Implementierung folgt als Cargo-Projekt in diesem Verzeichnis:

- [x] `README.md`
- [x] `ARCHITECTURE.md`
- [ ] `model` – Puzzle, Variablen `p(x,y,n)`, Constraint-Generierung
- [ ] `solver` – LP-Backend (+ Branch & Bound für fraktionale Lösungen)
- [ ] `ui` – Konsolen-REPL (über `trait Ui` abstrahiert)
- [ ] `app` – Game-Loop, generisch über `Solver` und `Ui`

## Geplante Bedienung (Konsole)

```text
$ cargo run
> set 0 0 5     # Zeile 0, Spalte 0 = 5 (Given eintragen)
> set 0 1 3
> show          # Brett anzeigen: feste Werte + Wahrscheinlichkeiten
> solve         # LP lösen, eindeutige Zellen als Lösung übernehmen
> prob 4 4      # Wahrscheinlichkeitsverteilung p(4,4,n) einer Zelle anzeigen
> load 530070600600195342098000060800060003400803001700020006060000280000419005000080079
> clear         # alle Givens löschen
> quit
```

## Entwicklung

```bash
cargo test    # Unit- + Integrationstests
cargo run     # Konsolen-Version
```
