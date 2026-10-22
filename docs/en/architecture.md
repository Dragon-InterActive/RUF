# Architecture

RUF has four layers. Only the lowest one knows the operating system.

```
 ┌──────────────────────────────────────────────────────────────────────┐
 │ 4  Designer / Freya TD     ComponentRegistry · property model         │
 ├──────────────────────────────────────────────────────────────────────┤
 │ 3  Logic (rncl + ruf)      RnclButton … · Component · Widget          │
 ├──────────────────────────────────────────────────────────────────────┤
 │ 2  Core (ruf/src/core.rs)  registry · signals · geometry · timers     │
 ├──────────────────────────────────────────────────────────────────────┤
 │ 1  System layer            trait Backend                              │
 │    (ruf/src/backend/)      gtk.rs │ win32.rs │ cocoa.rs               │
 └──────────────────────────────────────────────────────────────────────┘
          GTK 4 (Linux)     Win32 (Windows)     AppKit (macOS)
```

## 1. System layer (`crates/ruf/src/backend/`)

One backend per operating system, selected via `cfg` at compile time:

| File | System | Binding | Container class |
|---|---|---|---|
| `gtk.rs` | Linux, BSD | `gtk4-rs` | `RufFixed` (subclass of `GtkWidget`) |
| `win32.rs` | Windows | `windows-rs` | window class `RufContainer` |
| `cocoa.rs` | macOS | `objc2` | `RufView` (subclass of `NSView`, flipped Y axis) |

All implement the `Backend` trait (`backend/mod.rs`) – so the compiler ensures every
backend provides the full feature set. Backends are deliberately **"dumb"**: they
create controls, set and read their state and report user actions to the core
(`core::notify`, `core::close_requested`, `core::timer_fired`, `core::shortcut_fired`).

Each backend has its own container that positions children absolutely and reports
its size – so `on_resized` and custom layouts work the same in windows, tab pages and
scroll areas on every system.

## 2. Core (`crates/ruf/src/core.rs`)

Everything that must behave identically everywhere exists only once:

- **Registry:** every widget has a number (`Id`) and a node with kind, native
  reference, parent/children, cached properties and callbacks.
- **Lifetime:** destroyed widgets leave the registry. References to them become
  "dead": calls are no-ops, getters return defaults – no dangling pointers, no crashes.
- **Signals:** after every change – by code or by the user – `sync()` compares the
  native state with the last reported one and invokes callbacks exactly when
  something changed. Signals therefore fire the same everywhere, also for changes by
  code, and never twice.
- **Shared semantics:** radio buttons exclusive per parent; a combo box selects its
  first item; the selection follows inserts/removals; ranges clamp values; closing a
  window only hides it; `exec()` ends after the last visible window.
- **Timers, shortcuts, deferred deletion, panic protection** (a panic in a callback is
  caught, the remaining handlers keep running).

Rule between core and backend: while the core calls a backend (`setter`), it ignores
that backend's state notifications and synchronises itself afterwards.

### Signals → Rust closures

```
user clicks ─► native signal ─► backend: core::notify(id, Clicked)
                                     └─► core: sync(id) ─► toggled? ─► callbacks
                                                     └──► fire_clicked ─► callbacks
code: check.set_checked(true) ─► core ─► backend (notifications ignored) ─► sync(id) ─► callbacks
```

## 3. Logic layer (`crates/ruf/src`, `crates/rncl`)

### `Widget` – the safe reference

`Widget` is just a number plus an ownership flag:

- A widget **without** a parent (window) is freed when the `Widget` value that
  created it is dropped.
- A widget **with** a parent belongs to its parent.
- `clone()` creates a non-owning reference – ideal for closures.

### `Component` – the contract

Every RNCL component implements `ruf::Component`. Only `widget()` and `type_name()`
are required; position, size, visibility and the base properties come as default
methods. Own properties are added via `properties()`, `property()` and
`set_property()`.

### Threads

`Widget` and all components are **not `Send`/`Sync`** – the compiler prevents using
widgets from other threads. For background work there is `ruf::GuiSender`: the
sender is `Send` and forwards values to a handler on the GUI thread (GTK: idle source,
Win32: `PostMessage` to a message window, macOS: Grand Central Dispatch). See
[Components → Services](components.md#services-for-dynamic-user-interfaces).

## 4. Designer layer

Freya TD creates components only through the `ComponentRegistry` (by type name) and
edits them via the property model. See [Designer integration](designer.md).

## Toolkit version at runtime

```rust
let toolkit = ruf::toolkit();      // e.g. "GTK 4.24.1", "Win32 Windows 10 Pro (Build 19045)"
println!("{} {}", toolkit.name, toolkit.version);
```

## Minimum versions

| System | Minimum | Reason |
|---|---|---|
| GTK | 4.12 | `CssProvider::load_from_string`, `AlertDialog` |
| Windows | 10 | `GetDpiForSystem`, Common Controls 6 |
| macOS | 11 | current AppKit APIs, Apple Silicon |
