# Getting started

RUF uses **GTK 4** (≥ 4.12, binding `gtk4-rs`) on Linux, Windows (≥ 10) and macOS
(≥ 11). There is no C++, no Qt and no feature switch – the same code runs on all three
systems. Releases for Windows and macOS ship the GTK runtime (see [Release](release.md)).

## Requirements

| System | What | Installation |
|---|---|---|
| all | Rust ≥ 1.99 (edition 2024) | `rustup update` |
| Linux | GTK 4 with development files, `pkg-config` | see below |
| Windows | MSYS2 (UCRT64) with GTK 4, Rust target `x86_64-pc-windows-gnu` | see below |
| macOS | Xcode Command Line Tools, Homebrew with GTK 4 | `xcode-select --install`, `brew install gtk4 gtksourceview5` |

GTK on Linux:

```bash
# Arch / CachyOS
sudo pacman -S gtk4 gtksourceview5
# Debian / Ubuntu (≥ 24.04)
sudo apt install libgtk-4-dev libgtksourceview-5-dev
# Fedora
sudo dnf install gtk4-devel gtksourceview5-devel
```

## Build and test

```bash
cargo build --workspace
cargo test --workspace                     # Windows, macOS, Linux with a display
xvfb-run -a cargo test --workspace         # Linux without a display (CI, servers)
```

> On Linux in a Wayland session with `xvfb-run`, set `GDK_BACKEND=x11`, otherwise GTK
> tries to use the real Wayland session.

### Windows

Build on Windows in the MSYS2 shell "UCRT64" (<https://www.msys2.org>):

```bash
pacman -S --needed mingw-w64-ucrt-x86_64-gtk4 mingw-w64-ucrt-x86_64-gtksourceview5 mingw-w64-ucrt-x86_64-pkgconf \
                   mingw-w64-ucrt-x86_64-gcc
rustup target add x86_64-pc-windows-gnu      # rustup installed on Windows
cargo build --release --target x86_64-pc-windows-gnu
```

### macOS

```bash
brew install gtk4 gtksourceview5
cargo build --release
```

Cross-building from Linux is not intended: GTK needs the libraries of the target
system. For shipping see [Release](release.md).

## Examples

```bash
cargo run -p rncl --example hello              # minimal example
cargo run -p rncl --example counter            # counter with shared state
cargo run -p rncl --example form               # login form with validation
cargo run -p rncl --example gallery            # every component, wired up
cargo run -p rncl --example custom_component   # own component + registry
cargo run -p rncl --example designer           # form from a text description
cargo run -p rncl --example designer -- --dump # print all properties
```

## Your first program

```bash
cargo new my_program && cd my_program
```

`Cargo.toml`:

```toml
[dependencies]
ruf  = { version = "0.2.0" }   # the native foundation
rncl = { version = "0.2.0" }   # the native system widgets
# As long as the crates are unpublished, use paths:
# ruf  = { path = "../RUF/crates/ruf" }
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
2. **Keep the window alive** – the owning `RnclWindow` must exist until `exec()`.
3. **Clone for closures** – `component.clone()` is a cheap, non-owning reference.
4. **Everything on the main thread** – components are deliberately not `Send`;
   background work reports back via `ruf::GuiSender`.
5. **`use rncl::prelude::*`** – brings the `Component` trait for `set_position()` & co.

## Platform differences worth knowing

- **Sizes:** GTK never makes controls smaller than their natural size; `set_size`
  acts as a minimum there. Use `adjust_size()` (and `size()` afterwards) for layouts
  instead of fixed widths for captions.
- **Window position:** GTK 4 (Wayland) does not let programs position windows;
  `set_position` on a window is only stored.
- **Asynchronous layout:** GTK distributes sizes via its frame clock.
  `Application::process_events()` waits until layout has settled; `on_resized`
  reports the actual size.
- **Styling:** `set_style_sheet` understands a CSS subset (`color`, `background`,
  `font-weight`), see [Components](components.md#styling).

## Without a display (CI, servers, AI agents)

```bash
xvfb-run -a cargo run -p rncl --example designer -- --screenshot form.png   # Linux
```

In code: `window.widget().save_screenshot("image.png")` – also works for windows not
shown yet (they are shown briefly for the capture).
