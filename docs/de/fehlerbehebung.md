# Fehlerbehebung

## Build

**`Could not find Qt installation`**
Qt-Entwicklungspakete installieren bzw. `QMAKE` setzen:
`export QMAKE=/pfad/zu/qmake6`.

**Linkerfehler `undefined symbol: ruf$cxxbridge1$…`**
`cxx` und `cxx-gen` haben im `Cargo.lock` unterschiedliche Versionen. Angleichen:

```bash
cargo tree -i cxx | head -1          # z. B. cxx v1.0.205
cargo update -p cxx-gen --precise 0.7.205
```

**`error[cxxqt]: cannot parse string into token stream`**
Syntaxfehler in einer Bridge-Datei (`src/ffi.rs`). cxx-qt liest die Datei vor rustc –
die Stelle mit `cargo check` nach einem Zurücksetzen der letzten Änderung eingrenzen.

**`ruf benötigt ein Backend-Feature`**
`ruf` ohne Features eingebunden: `features = ["qt6"]` ergänzen.

## Laufzeit

**Panik: „Vor dem ersten Widget muss eine ruf::Application erzeugt werden"**
`Application::new()` als erste Zeile in `main`.

**Panik: „Es darf nur eine Application pro Prozess existieren"**
Qt erlaubt nur eine. In Tests: eine Testdatei mit `harness = false` und eigenem `main()`
(Vorlage `crates/rncl/tests/components.rs`).

**Fenster erscheint kurz und verschwindet**
Das besitzende `RnclWindow` wurde gedroppt (z. B. in einer Hilfsfunktion erzeugt und
nicht zurückgegeben). Fenster in `main` halten oder aus der Funktion zurückgeben.

**Komponente reagiert nicht mehr / Getter liefern leere Werte**
Das Widget wurde von Qt gelöscht (Elternfenster geschlossen und freigegeben).
`komponente.widget().is_alive()` prüfen.

**`qt.qpa.xcb: could not connect to display`**
Kein Bildschirm verfügbar (SSH, CI). `QT_QPA_PLATFORM=offscreen` setzen.

**Meldung „ruf: Panik in einem Signal-Callback abgefangen"**
Eine Closure ist in Panik geraten. RUF fängt das ab, damit die Anwendung weiterläuft;
die Ursache steht in der Meldung.
