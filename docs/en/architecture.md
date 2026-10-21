# Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ 3. Form designer (Freya TD)                                 │
│    knows only: ComponentRegistry · Component · PropertyValue│
├─────────────────────────────────────────────────────────────┤
│ 2. Logic layer – RNCL components (pure Rust)                │
│    RnclWindow · RnclButton · … · custom components          │
│    ruf::Component · ruf::Widget · property model            │
├─────────────────────────────────────────────────────────────┤
│ 1. System layer – the bridge                                │
│    ruf::ffi (#[cxx_qt::bridge])  ⇄  cpp/ruf_widgets.{h,cpp} │
│    cxx · cxx-qt · cxx-qt-lib     ⇄  installed Qt ≥ 6        │
└─────────────────────────────────────────────────────────────┘
```

## 1. System layer (`crates/ruf/cpp`, `crates/ruf/src/ffi.rs`)

- A **thin C++ layer** over QtWidgets: create widgets, read/write properties,
  connect signals. No application logic.
- The bridge is declared with `#[cxx_qt::bridge]` and compiled by `cxx-qt-build`;
  Qt types (`QString`, `QRect`) come from `cxx-qt-lib`.
- `build.rs` locates the installed Qt, links `Core`, `Gui`, `Widgets` and exports the
  headers, so downstream crates can write their own bridges against `ruf`.

### Signals → Rust closures

```
QPushButton::clicked ─► C++ lambda ─► invoke_callback(&RufCallback) ─► Box<dyn FnMut()>
```

- The closure lives in a `RefCell`: if it (indirectly) re-emits the same signal, the
  nested call is dropped instead of causing undefined behaviour.
- **Panics** inside closures are caught and reported on `stderr` – they never cross the
  C++ boundary, and the event loop keeps running.
- Connections are tied to the widget: when it is destroyed, the closure is freed too.

## 2. Logic layer (`crates/ruf/src`, `crates/rncl`)

### `Widget` – the safe reference

`ruf::Widget` holds a `QPointer` internally. This gives the **ownership model**:

| Situation | Behaviour |
|---|---|
| widget without parent (window) | deleted when the creating `Widget` is dropped |
| widget with parent | owned by its parent (Qt model); dropping it in Rust does not remove it |
| `widget.clone()` | non-owning reference – ideal for closures |
| Qt deleted the widget | all calls are no-ops, getters return defaults, `is_alive()` = `false` |

So there are **no dangling pointers** on the Rust side.

### `Component` – the contract

Every RNCL component implements `ruf::Component`. Only `widget()` and `type_name()`
are required; position, size, visibility and the base properties come as default
methods. Custom properties are added via `properties()`, `property()` and
`set_property()`.

### Threads

`Widget` and all components are **not `Send`/`Sync`** – the compiler prevents using
widgets from other threads (a Qt rule).

## 3. Designer layer

Freya TD creates components only through the `ComponentRegistry` (by type name) and
edits them through the property model. See [Designer integration](designer.md).

## Qt versions

RUF binds whichever Qt is installed – 6.x today, Qt 7 later as a separate feature.
Newer functionality is enabled in C++ via `QT_VERSION_CHECK` and always has a fallback.
Example – colour scheme:

| Function | requires | otherwise |
|---|---|---|
| `Application::color_scheme()` | Qt 6.5 | `ColorScheme::Unknown` |
| `Application::set_color_scheme()` | Qt 6.8 | returns `false` |

Check at runtime:

```rust
use ruf::QtVersion;

let qt = QtVersion::runtime();
if qt.at_least(6, 8) { /* use the new feature */ }
println!("built against {}, running with {qt}", QtVersion::compile_time());
```

## Features

| Feature | default | Meaning |
|---|---|---|
| `qt6` | yes | Qt ≥ 6 backend (QtWidgets). Without a backend the build stops with a clear message. |
