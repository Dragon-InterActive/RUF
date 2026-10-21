# Erste Schritte

## Voraussetzungen

| Was | Version | Hinweis |
|---|---|---|
| Rust | ≥ 1.99.1 (Edition 2024) | `rustup update` |
| Qt | ≥ 6.0, empfohlen ≥ 6.8 | Module Core, Gui, Widgets inkl. Header |
| C++-Compiler | C++17 | GCC, Clang oder MSVC |

Qt wird über `qmake6` (bzw. `qmake`) gefunden. Liegt es nicht im `PATH`:

```bash
export QMAKE=/opt/Qt/6.12.0/gcc_64/bin/qmake6
```

Paketbeispiele:

```bash
# Arch / CachyOS
sudo pacman -S qt6-base
# Debian / Ubuntu
sudo apt install qt6-base-dev
# Fedora
sudo dnf install qt6-qtbase-devel
```

## Bauen und testen

```bash
cargo build --workspace
cargo test --workspace      # Integrationstests laufen offscreen – kein Bildschirm nötig
```

> Der erste Build dauert etwas länger: cxx-qt generiert und kompiliert C++-Code und
> ruft Qts `moc` auf.

## Beispiele

```bash
cargo run -p rncl --example hello              # Minimalbeispiel
cargo run -p rncl --example counter            # Zähler mit gemeinsamem Zustand
cargo run -p rncl --example form               # Anmeldeformular mit Validierung
cargo run -p rncl --example gallery            # alle Komponenten, verdrahtet
cargo run -p rncl --example custom_component   # eigene Komponente + Registry
cargo run -p rncl --example designer           # Formular aus Textbeschreibung
cargo run -p rncl --example designer -- --dump # alle Eigenschaften ausgeben
```

## Das erste eigene Programm

```bash
cargo new mein_programm && cd mein_programm
```

`Cargo.toml`:

```toml
[dependencies]
ruf  = { version = "0.1.0", features = ["qt6"] }
rncl = { version = "0.1.0" }
# Solange die Crates nicht veröffentlicht sind, per Pfad:
# ruf  = { path = "../RUF/crates/ruf", features = ["qt6"] }
# rncl = { path = "../RUF/crates/rncl" }
```

`src/main.rs`:

```rust
use rncl::prelude::*;
use std::{cell::Cell, rc::Rc};

fn main() {
    let app = Application::new();

    let window = RnclWindow::new("Zähler");
    window.set_size(260, 120);

    let label = RnclLabel::new(&window, "0");
    label.set_geometry(20, 20, 220, 24);

    let plus = RnclButton::new(&window, "+1");
    plus.set_position(20, 60);

    let reset = RnclButton::new(&window, "Zurücksetzen");
    reset.set_position(120, 60);

    // Gemeinsamer Zustand für mehrere Closures
    let count = Rc::new(Cell::new(0));

    let (c, l) = (count.clone(), label.clone());
    plus.on_clicked(move || {
        c.set(c.get() + 1);
        l.set_text(&c.get().to_string());
    });

    let (c, l) = (count.clone(), label.clone());
    reset.on_clicked(move || {
        c.set(0);
        l.set_text("0");
    });

    window.show();
    std::process::exit(app.exec());
}
```

## Die fünf Regeln

1. **`Application` zuerst** – vor jeder Komponente, genau einmal pro Prozess.
2. **Fenster am Leben halten** – das besitzende `RnclWindow` muss bis `exec()` existieren.
3. **Klonen für Closures** – `komponente.clone()` ist ein leichter, nicht besitzender Verweis.
4. **Alles im Hauptthread** – Komponenten sind bewusst nicht `Send`.
5. **`use rncl::prelude::*`** – bringt den `Component`-Trait für `set_position()` & Co. mit.

## Ohne Bildschirm (CI, Server, KI-Agenten)

```bash
QT_QPA_PLATFORM=offscreen cargo run -p rncl --example designer -- --screenshot form.png
```

Im Code: `window.widget().save_screenshot("bild.png")` nach `window.show()` und
`Application::process_events()`.
