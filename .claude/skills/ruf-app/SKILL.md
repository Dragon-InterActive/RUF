---
name: ruf-app
description: Eine Desktop-Anwendung oder ein Formular mit RUF und RNCL-Komponenten bauen (Fenster, Buttons, Eingabefelder, Ereignisse), inkl. Sichtprüfung per Offscreen-Screenshot. Verwenden, wenn mit ruf/rncl eine Oberfläche erstellt, ein Beispiel geschrieben oder ein Fehler in einer RUF-App gesucht wird.
---

# Anwendung mit RUF/RNCL bauen

## Abhängigkeiten

```toml
[dependencies]
ruf  = { version = "0.1.0", features = ["qt6"] }
rncl = { version = "0.1.0" }
```

Voraussetzung: Qt >= 6 mit Entwicklungsdateien (`qmake6` im `PATH` oder `QMAKE=/pfad/zu/qmake6`), C++17-Compiler.

## Grundmuster

```rust
use rncl::prelude::*;

fn main() {
    let app = Application::new();                // 1. immer zuerst
    let window = RnclWindow::new("Titel");       // 2. Fenster
    window.set_size(400, 300);

    let label = RnclLabel::new(&window, "…");    // 3. Kinder: Position relativ zum Elternteil
    label.set_geometry(20, 20, 360, 24);

    let button = RnclButton::new(&window, "OK");
    button.set_position(20, 60);
    let l = label.clone();                       // 4. Klon für die Closure
    button.on_clicked(move || l.set_text("Geklickt"));

    window.show();                               // 5. anzeigen
    std::process::exit(app.exec());              // 6. Event-Loop
}
```

## Komponenten (Stand 0.1)

| Typ | Konstruktor | Wichtigste API |
|---|---|---|
| `RnclWindow` | `new(title)` | `set_title`, `close` |
| `RnclPanel` | `new(&parent)` | Container, Positionen relativ |
| `RnclButton` | `new(&parent, text)` | `set_text`, `on_clicked`, `click` |
| `RnclLabel` | `new(&parent, text)` | `set_text` (einfaches HTML erlaubt) |
| `RnclTextField` | `new(&parent)` | `set_text`, `set_placeholder`, `set_password_mode`, `on_text_changed`, `on_return_pressed` |
| `RnclCheckBox` | `new(&parent, text)` | `set_checked`, `is_checked`, `on_toggled`, `click` |

Alle (Trait `Component`, kommt mit der prelude): `set_position`, `set_size`,
`set_geometry`, `show`, `hide`, `set_enabled`, `set_tool_tip`, `set_style_sheet`,
`set_name`, `property`/`set_property`.

## Regeln, die Fehler vermeiden

- `Application` vor jeder Komponente erzeugen, sonst Panik mit klarer Meldung.
- Nur **eine** `Application` pro Prozess; alles im Hauptthread.
- Das besitzende `RnclWindow` am Leben halten (Variable in `main`). Wird es
  gedroppt, verschwindet das Fenster samt Kindern. Kinder-Variablen dürfen
  gedroppt werden – die Widgets bleiben (Qt-Elternteil besitzt sie).
- Closures brauchen `'static`: Komponenten per `.clone()` einfangen, eigenen
  Zustand per `Rc<Cell<_>>`/`Rc<RefCell<_>>` teilen.
- Signale feuern auch bei programmatischen Änderungen (`set_text`, `set_checked`).
- `Application::quit()`, `Application::set_color_scheme(..)` sind assoziierte Funktionen
  und daher aus jeder Closure aufrufbar.

## Sichtprüfung ohne Bildschirm (für KI-Agenten)

```rust
window.show();
Application::process_events();
window.widget().save_screenshot("/tmp/ui.png");
```

```bash
QT_QPA_PLATFORM=offscreen cargo run --example mein_beispiel
```

Danach das PNG ansehen und Layout (Überlappungen, abgeschnittene Texte) prüfen.
Fertiges Beispiel: `cargo run -p rncl --example designer -- --screenshot /tmp/form.png`.

## Tests

Qt-Tests brauchen den Hauptthread: Testdatei mit `harness = false` und eigenem
`main()`, `Application::with_args(["test", "-platform", "offscreen"])`.
Vorlage: `crates/rncl/tests/components.rs`.
