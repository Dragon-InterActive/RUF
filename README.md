# RUF – Rust native UI Framework

Desktop-UI-Framework für Rust auf Basis von **GTK 4** – auf Linux, Windows und macOS
(Releases für Windows/macOS liefern die GTK-Laufzeit mit, siehe `tools/` und
`docs/de/release.md`) – Fundament des RAD-Tools **Freya TD**.
*Desktop UI framework for Rust based on **GTK 4** – on Linux, Windows and macOS
(Windows/macOS releases ship the GTK runtime, see `tools/` and `docs/en/release.md`) –
the foundation of the RAD tool **Freya TD**.*

| Crate | Rolle / Role |
| --- | --- |
| [`ruf`](crates/ruf) | System-Ebene (GTK-4-Backend hinter dem Trait `Backend`), plattformunabhängiger Kern und Logik-Basis: `Widget`, `Component`, `ComponentRegistry`, Eigenschaftsmodell |
| [`rncl`](crates/rncl) | Rust native Component Library: 16 Standardkomponenten – Fenster, Container, Registerkarten, Scrollbereich, Buttons, Text-, Listen- und Zahleneingaben, Code-Editor, Fortschrittsanzeige |

```toml
[dependencies]
ruf  = { version = "0.2.0" } # Das native Fundament – Backend wird automatisch gewählt
rncl = { version = "0.2.0" } # Die nativen System-Widgets
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
cargo test --workspace                 # Tests gegen das echte Toolkit (Linux ohne Bildschirm: xvfb-run -a …)
cargo run -p rncl --example hello      # weitere: counter, form, gallery, custom_component, designer
```

## Dokumentation / Documentation

- 🇩🇪 [Deutsch](docs/de/README.md)
- 🇬🇧 [English](docs/en/README.md)

## KI-Unterstützung / AI support

Skills für Claude Code unter [`.claude/skills/`](.claude/skills), kompakte Regeln für
andere Assistenten in [`AGENTS.md`](AGENTS.md).

## Lizenz / License

Nach Wahl lizenziert unter / Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

Programme, die damit gebaut werden, dürfen unter beliebiger Lizenz stehen – auch
proprietär. Beiträge werden, sofern nicht ausdrücklich anders angegeben, unter
denselben beiden Lizenzen eingebracht.
*Software built with it may use any license, including proprietary ones. Unless
explicitly stated otherwise, contributions are dual-licensed as above.*
