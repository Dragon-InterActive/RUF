# RUF – Rust native UI Framework

Natives Desktop-UI-Framework für Rust mit den **Steuerelementen des Systems** –
GTK 4 (Linux), Win32 (Windows), Cocoa (macOS) – Fundament des RAD-Tools **Freya TD**.
*Native desktop UI framework for Rust using the **system's own controls** – GTK 4
(Linux), Win32 (Windows), Cocoa (macOS) – the foundation of the RAD tool **Freya TD**.*

| Crate | Rolle / Role |
| --- | --- |
| [`ruf`](crates/ruf) | System-Ebene (Backends für GTK 4, Win32, Cocoa), plattformunabhängiger Kern und Logik-Basis: `Widget`, `Component`, `ComponentRegistry`, Eigenschaftsmodell |
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

Apache-2.0
