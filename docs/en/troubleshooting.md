# Troubleshooting

## Build

**`Could not find Qt installation`**
Install the Qt development packages or set `QMAKE`:
`export QMAKE=/path/to/qmake6`.

**Linker error `undefined symbol: ruf$cxxbridge1$…`**
`cxx` and `cxx-gen` have different versions in `Cargo.lock`. Align them:

```bash
cargo tree -i cxx | head -1          # e.g. cxx v1.0.205
cargo update -p cxx-gen --precise 0.7.205
```

**`error[cxxqt]: cannot parse string into token stream`**
Syntax error in a bridge file (`src/ffi.rs`). cxx-qt reads the file before rustc –
narrow it down by reverting the last change and running `cargo check`.

**`ruf benötigt ein Backend-Feature`** ("ruf needs a backend feature")
`ruf` was added without features: add `features = ["qt6"]`.

## Runtime

**Panic: "Vor dem ersten Widget muss eine ruf::Application erzeugt werden"**
("an Application must be created before the first widget") – make
`Application::new()` the first line of `main`.

**Panic: "Es darf nur eine Application pro Prozess existieren"**
("only one Application per process") – Qt allows just one. For tests: one test file
with `harness = false` and its own `main()` (template `crates/rncl/tests/components.rs`).

**Window flashes and disappears**
The owning `RnclWindow` was dropped (e.g. created in a helper function and not
returned). Keep the window in `main` or return it from the function.

**Component no longer reacts / getters return empty values**
Qt deleted the widget (parent window closed and freed). Check
`component.widget().is_alive()`.

**`qt.qpa.xcb: could not connect to display`**
No display available (SSH, CI). Set `QT_QPA_PLATFORM=offscreen`.

**Message "ruf: Panik in einem Signal-Callback abgefangen"**
("caught a panic in a signal callback") – a closure panicked. RUF catches it so the
application keeps running; the cause is included in the message.
