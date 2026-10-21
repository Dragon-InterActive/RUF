# RUF – Rust native UI Framework

Natives Desktop-UI-Framework für Rust auf Basis von **Qt ≥ 6** – Fundament des
RAD-Tools **Freya TD**.
*Native desktop UI framework for Rust built on **Qt ≥ 6** – the foundation of the
RAD tool **Freya TD**.*

| Crate | Rolle / Role |
|---|---|
| [`ruf`](crates/ruf) | System-Ebene (cxx-qt-Brücke) und Logik-Basis: `Widget`, `Component`, `ComponentRegistry`, Eigenschaftsmodell |
| [`rncl`](crates/rncl) | Rust native Component Library: `RnclWindow`, `RnclPanel`, `RnclButton`, `RnclLabel`, `RnclTextField`, `RnclCheckBox` |

```toml
[dependencies]
ruf  = { version = "0.1.0", features = ["qt6"] } # Das native Fundament
rncl = { version = "0.1.0" }                    # Die nativen System-Widgets
```

```rust
use rncl::prelude::*;

fn main() {
    let app = Application::new();
    let window = RnclWindow::new("Hallo RUF");
    window.set_size(320, 140);

    let button = RnclButton::new(&window, "Klick mich");
    button.set_position(20, 50);
    let b = button.clone();
    button.on_clicked(move || b.set_text("Danke!"));

    window.show();
    std::process::exit(app.exec());
}
```

```bash
cargo test --workspace                 # Tests gegen echtes Qt (offscreen)
cargo run -p rncl --example hello      # weitere: counter, form, custom_component, designer
```

## Dokumentation / Documentation

- 🇩🇪 [Deutsch](docs/de/README.md)
- 🇬🇧 [English](docs/en/README.md)

## KI-Unterstützung / AI support

Skills für Claude Code unter [`.claude/skills/`](.claude/skills), kompakte Regeln für
andere Assistenten in [`AGENTS.md`](AGENTS.md).

## Lizenz / License

MIT OR Apache-2.0
