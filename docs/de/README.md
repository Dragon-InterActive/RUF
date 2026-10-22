# RUF & RNCL – Entwickler-Dokumentation

🇬🇧 [English version](../en/README.md)

**RUF** (Rust native UI Framework) ist das native Fundament für Desktop-Oberflächen
in Rust mit den **nativen Steuerelementen des Systems** – GTK 4 unter Linux, Win32
unter Windows, Cocoa unter macOS. **RNCL** (Rust native Component Library) liefert
darauf die UI-Elemente – Fenster, Buttons, Textfelder, Checkboxen … – als
eigenständige, austauschbare Komponenten. Beide bilden die Basis des RAD-Tools
**Freya TD**.

```toml
[dependencies]
ruf  = { version = "0.2.0" } # Das native Fundament (GTK 4 / Win32 / Cocoa – automatisch)
rncl = { version = "0.2.0" } # Die nativen System-Widgets
```

## Inhalt

| Kapitel | Worum es geht |
|---|---|
| [Erste Schritte](erste-schritte.md) | Installation, erstes Fenster, Beispiele ausführen |
| [Architektur](architektur.md) | Die Ebenen, Backends, Kern, Besitzmodell, Threads |
| [Komponenten](komponenten.md) | Referenz aller RNCL-Komponenten |
| [Eigene Komponenten](eigene-komponenten.md) | Eigene RNCL-Komponenten schreiben und registrieren |
| [Designer-Anbindung](designer.md) | Registry und Eigenschaftsmodell für Freya TD |
| [KI-Assistenz](ki-assistenz.md) | Skills und Arbeitsweise mit KI-Assistenten |
| [Fehlerbehebung](fehlerbehebung.md) | Typische Build- und Laufzeitprobleme |

## Auf einen Blick

```rust
use rncl::prelude::*;

fn main() {
    let app = Application::new();

    let window = RnclWindow::new("Hallo RUF");
    window.set_size(320, 140);

    let label = RnclLabel::new(&window, "Noch nicht geklickt");
    label.set_geometry(20, 20, 280, 24);

    let button = RnclButton::new(&window, "Klick mich");
    button.set_position(20, 70);

    let l = label.clone();
    button.on_clicked(move || l.set_text("Geklickt!"));

    window.show();
    std::process::exit(app.exec());
}
```

## Repository-Aufbau

```
RUF/
├── Cargo.toml                 Workspace
├── crates/
│   ├── ruf/                   Fundament
│   │   └── src/
│   │       ├── backend/       System-Ebene: gtk.rs (Linux), win32.rs (Windows), cocoa.rs (macOS)
│   │       ├── core.rs        Kern: Registry, Signale, Geometrie, Timer
│   │       └── …              Widget, Component, Registry, Eigenschaften, Stil
│   └── rncl/                  Standardkomponenten
│       ├── src/               RnclWindow, RnclButton, …
│       ├── examples/          hello, counter, form, gallery, custom_component, designer
│       └── tests/             Integrationstests gegen das echte Toolkit
├── docs/{de,en}/              diese Dokumentation
└── .claude/skills/            Skills für KI-Assistenten
```
