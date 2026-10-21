# RUF & RNCL – Developer Documentation

🇩🇪 [Deutsche Fassung](../de/README.md)

**RUF** (Rust native UI Framework) is the native foundation for desktop user
interfaces in Rust, built on **Qt ≥ 6**. **RNCL** (Rust native Component Library)
provides the UI elements on top of it – windows, buttons, text fields, check boxes … –
as self-contained, interchangeable components. Together they form the basis of the
RAD tool **Freya TD**.

```toml
[dependencies]
ruf  = { version = "0.1.0", features = ["qt6"] } # the native foundation
rncl = { version = "0.1.0" }                    # the native system widgets
```

## Contents

| Chapter | About |
|---|---|
| [Getting started](getting-started.md) | Installation, first window, running the examples |
| [Architecture](architecture.md) | The three layers, ownership model, threads, Qt versions |
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
│   ├── ruf/                   system layer + logic foundation
│   │   ├── cpp/               C++ shim over QtWidgets
│   │   └── src/               ffi (bridge), Widget, Component, Registry …
│   └── rncl/                  standard components
│       ├── src/               RnclWindow, RnclButton, …
│       ├── examples/          hello, counter, form, custom_component, designer
│       └── tests/             integration tests against real Qt (offscreen)
├── docs/{de,en}/              this documentation
└── .claude/skills/            skills for AI assistants
```

> Note: doc comments and UI strings in the source code are written in German.
