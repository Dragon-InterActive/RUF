# Erste Schritte

RUF nutzt **GTK 4** (≥ 4.12, Bindung `gtk4-rs`) auf Linux, Windows (≥ 10) und
macOS (≥ 11). Es gibt kein C++, kein Qt und keine Feature-Schalter – derselbe Code
läuft auf allen drei Systemen. Releases für Windows und macOS liefern die
GTK-Laufzeit mit (siehe [Release](release.md)).

## Voraussetzungen

| System | Was | Installation |
|---|---|---|
| alle | Rust ≥ 1.99 (Edition 2024) | `rustup update` |
| Linux | GTK 4 mit Entwicklungsdateien, `pkg-config` | siehe unten |
| Windows | MSYS2 (UCRT64) mit GTK 4, Rust-Ziel `x86_64-pc-windows-gnu` | siehe unten |
| macOS | Xcode Command Line Tools, Homebrew mit GTK 4 | `xcode-select --install`, `brew install gtk4` |

GTK unter Linux:

```bash
# Arch / CachyOS
sudo pacman -S gtk4
# Debian / Ubuntu (≥ 24.04)
sudo apt install libgtk-4-dev
# Fedora
sudo dnf install gtk4-devel
```

## Bauen und testen

```bash
cargo build --workspace
cargo test --workspace                     # Windows, macOS, Linux mit Bildschirm
xvfb-run -a cargo test --workspace         # Linux ohne Bildschirm (CI, Server)
```

> Unter Linux mit Wayland-Sitzung und `xvfb-run`: `GDK_BACKEND=x11` setzen, sonst
> versucht GTK die echte Wayland-Sitzung zu benutzen.

### Windows

Gebaut wird unter Windows in der MSYS2-Shell „UCRT64“ (<https://www.msys2.org>):

```bash
pacman -S --needed mingw-w64-ucrt-x86_64-gtk4 mingw-w64-ucrt-x86_64-pkgconf \
                   mingw-w64-ucrt-x86_64-gcc
rustup target add x86_64-pc-windows-gnu      # rustup unter Windows installiert
cargo build --release --target x86_64-pc-windows-gnu
```

### macOS

```bash
brew install gtk4
cargo build --release
```

Ein Cross-Build von Linux aus ist nicht vorgesehen: GTK braucht die Bibliotheken des
Zielsystems. Ausliefern siehe [Release](release.md).

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
ruf  = { version = "0.2.0" }   # das native Fundament
rncl = { version = "0.2.0" }   # die nativen System-Widgets
# Solange die Crates nicht veröffentlicht sind, per Pfad:
# ruf  = { path = "../RUF/crates/ruf" }
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
4. **Alles im Hauptthread** – Komponenten sind bewusst nicht `Send`; Hintergrundarbeit
   meldet sich per `ruf::GuiSender` zurück.
5. **`use rncl::prelude::*`** – bringt den `Component`-Trait für `set_position()` & Co. mit.

## Plattformunterschiede, die man kennen sollte

- **Größen:** GTK macht Steuerelemente nie kleiner als ihre natürliche Größe;
  `set_size` wirkt dort als Mindestgröße. Für Layouts deshalb `adjust_size()` bzw.
  `size()` nach `adjust_size()` verwenden statt fester Breiten für Beschriftungen.
- **Fensterposition:** GTK 4 (Wayland) erlaubt Programmen nicht, Fenster zu
  positionieren; `set_position` eines Fensters wird nur gespeichert.
- **Asynchrones Layout:** Unter GTK verteilt das Toolkit Größen über seine
  Frame-Clock. `Application::process_events()` wartet, bis das Layout zur Ruhe
  gekommen ist; `on_resized` meldet die tatsächliche Größe.
- **Stil:** `set_style_sheet` versteht eine CSS-Teilmenge (`color`, `background`,
  `font-weight`), siehe [Komponenten](komponenten.md#gestaltung).

## Ohne Bildschirm (CI, Server, KI-Agenten)

```bash
xvfb-run -a cargo run -p rncl --example designer -- --screenshot form.png   # Linux
```

Im Code: `window.widget().save_screenshot("bild.png")` – funktioniert auch bei noch
nicht angezeigten Fenstern (sie werden dafür kurz eingeblendet).
