# RUF & RNCL – Developer Documentation

🇩🇪 [Deutsche Fassung](../de/README.md)

**RUF** (Rust native UI Framework) is the native foundation for desktop user
interfaces in Rust using the **native controls of the system** – GTK 4 on Linux,
Win32 on Windows, Cocoa on macOS. **RNCL** (Rust native Component Library)
provides the UI elements on top of it – windows, buttons, text fields, check boxes … –
as self-contained, interchangeable components. Together they form the basis of the
RAD tool **Freya TD**.

```toml
[dependencies]
ruf  = { version = "0.2.0" } # the native foundation (GTK 4 / Win32 / Cocoa – automatic)
rncl = { version = "0.2.0" } # the native system widgets
```

## Contents

| Chapter | About |
|---|---|
| [Getting started](getting-started.md) | Installation, first window, running the examples |
| [Architecture](architecture.md) | Layers, backends, core, ownership model, threads |
| [Components](components.md) | Reference of all RNCL components |
| [Custom components](custom-components.md) | Writing and registering your own RNCL components |
| [Designer integration](designer.md) | Registry and property model for Freya TD |
| [AI assistance](ai-assistance.md) | Skills and working with AI assistants |
| [Troubleshooting](troubleshooting.md) | Common build and runtime problems |

## At a glance

```rust
use rncl::prelude::*;

fn main() {
    let app = Application::new();

    let window = RnclWindow::new("Hello RUF");
    window.set_size(320, 140);

    let label = RnclLabel::new(&window, "Not clicked yet");
    label.set_geometry(20, 20, 280, 24);

    let button = RnclButton::new(&window, "Click me");
    button.set_position(20, 70);

    let l = label.clone();
    button.on_clicked(move || l.set_text("Clicked!"));

    window.show();
    std::process::exit(app.exec());
}
```

## Repository layout

```
RUF/
├── Cargo.toml                 workspace
├── crates/
│   ├── ruf/                   foundation
│   │   └── src/
│   │       ├── backend/       system layer: gtk.rs (Linux), win32.rs (Windows), cocoa.rs (macOS)
│   │       ├── core.rs        core: registry, signals, geometry, timers
│   │       └── …              Widget, Component, registry, properties, style
│   └── rncl/                  standard components
│       ├── src/               RnclWindow, RnclButton, …
│       ├── examples/          hello, counter, form, gallery, custom_component, designer
│       └── tests/             integration tests against the real toolkit
├── docs/{de,en}/              this documentation
└── .claude/skills/            skills for AI assistants
```

> Note: doc comments and UI strings in the source code are written in German.
