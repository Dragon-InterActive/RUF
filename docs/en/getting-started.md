# Getting started

## Requirements

| What | Version | Note |
|---|---|---|
| Rust | ≥ 1.88 (edition 2024) | `rustup update` |
| Qt | ≥ 6.0, ≥ 6.8 recommended | modules Core, Gui, Widgets incl. headers |
| C++ compiler | C++17 | GCC, Clang or MSVC |

Qt is located via `qmake6` (or `qmake`). If it is not on your `PATH`:

```bash
export QMAKE=/opt/Qt/6.12.0/gcc_64/bin/qmake6
```

Package examples:

```bash
# Arch / CachyOS
sudo pacman -S qt6-base
# Debian / Ubuntu
sudo apt install qt6-base-dev
# Fedora
sudo dnf install qt6-qtbase-devel
```

## Build and test

```bash
cargo build --workspace
cargo test --workspace      # integration tests run offscreen – no display needed
```

> The first build takes a little longer: cxx-qt generates and compiles C++ code and
> runs Qt's `moc`.

## Examples

```bash
cargo run -p rncl --example hello              # minimal example
cargo run -p rncl --example counter            # counter with shared state
cargo run -p rncl --example form               # login form with validation
cargo run -p rncl --example custom_component   # custom component + registry
cargo run -p rncl --example designer           # form built from a text description
cargo run -p rncl --example designer -- --dump # print all properties
```

## Your first program

```bash
cargo new my_app && cd my_app
```

`Cargo.toml`:

```toml
[dependencies]
ruf  = { version = "0.1.0", features = ["qt6"] }
rncl = { version = "0.1.0" }
# Until the crates are published, use paths:
# ruf  = { path = "../RUF/crates/ruf", features = ["qt6"] }
# rncl = { path = "../RUF/crates/rncl" }
```

`src/main.rs`:

```rust
use rncl::prelude::*;
use std::{cell::Cell, rc::Rc};

fn main() {
    let app = Application::new();

    let window = RnclWindow::new("Counter");
    window.set_size(260, 120);

    let label = RnclLabel::new(&window, "0");
    label.set_geometry(20, 20, 220, 24);

    let plus = RnclButton::new(&window, "+1");
    plus.set_position(20, 60);

    let reset = RnclButton::new(&window, "Reset");
    reset.set_position(120, 60);

    // Shared state for several closures
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

## The five rules

1. **`Application` first** – before any component, exactly once per process.
2. **Keep the window alive** – the owning `RnclWindow` must exist until `exec()` returns.
3. **Clone for closures** – `component.clone()` is a lightweight, non-owning reference.
4. **Everything on the main thread** – components are intentionally not `Send`.
5. **`use rncl::prelude::*`** – brings the `Component` trait for `set_position()` & co.

## Without a display (CI, servers, AI agents)

```bash
QT_QPA_PLATFORM=offscreen cargo run -p rncl --example designer -- --screenshot form.png
```

In code: `window.widget().save_screenshot("image.png")` after `window.show()` and
`Application::process_events()`.
