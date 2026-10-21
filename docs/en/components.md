# Component reference

All components implement `ruf::Component` and therefore provide:

| Method | Description |
|---|---|
| `set_position(x, y)` / `position()` | position relative to the parent |
| `set_size(w, h)` / `size()` | size in pixels |
| `set_geometry(x, y, w, h)` / `geometry()` | both at once |
| `show()` / `hide()` / `is_visible()` | visibility |
| `set_enabled(b)` / `is_enabled()` | interactivity |
| `set_tool_tip(text)` | tooltip |
| `set_style_sheet(css)` | Qt style sheet, e.g. `"color: red; font-weight: bold;"` |
| `set_name(n)` / `name()` | unique identifier (`objectName`) |
| `widget()` | the underlying `ruf::Widget` |
| `properties()` / `property(n)` / `set_property(n, v)` | property model |

**Base properties** (every component): `name`, `x`, `y`, `width`, `height`,
`visible`, `enabled`, `tool_tip`, `style_sheet`.

Every component has two constructors: `new(&parent, …)` and
`with_parent(Option<&Widget>, …)` (for registry factories and composite components).

---

## `RnclWindow` – window

Qt: `QWidget` (top-level) · container · category *Container*

```rust
let window = RnclWindow::new("Title");   // default size 640×480, initially hidden
window.set_size(400, 300);
window.show();
window.close();                          // last window closed → exec() returns
```

| Property | Type | |
|---|---|---|
| `title` | Text | window title |

## `RnclPanel` – container

Qt: `QWidget` · container · category *Container*

```rust
let panel = RnclPanel::new(&window);
panel.set_geometry(10, 10, 300, 100);
let ok = RnclButton::new(&panel, "OK");  // position relative to the panel
```

## `RnclButton` – push button

Qt: `QPushButton` · category *Standard*

```rust
let button = RnclButton::new(&window, "Save");
button.on_clicked(|| println!("saved"));
button.click();                          // click programmatically
```

| Property | Type | |
|---|---|---|
| `text` | Text | caption |

## `RnclLabel` – text

Qt: `QLabel` · category *Standard*

```rust
let label = RnclLabel::new(&window, "Status: <b>ready</b>"); // simple HTML
label.set_text("Done");
```

| Property | Type | |
|---|---|---|
| `text` | Text | displayed text |

## `RnclTextField` – line edit

Qt: `QLineEdit` · category *Eingabe* (input)

```rust
let field = RnclTextField::new(&window);
field.set_placeholder("e-mail");
field.on_text_changed(|t| println!("new: {t}"));
field.on_return_pressed(|| println!("Enter"));

let pass = RnclTextField::new(&window);
pass.set_password_mode(true);
```

| Property | Type | |
|---|---|---|
| `text` | Text | entered text |
| `placeholder` | Text | hint while empty |
| `password` | Bool | hide input |

## `RnclCheckBox` – check box

Qt: `QCheckBox` · category *Eingabe* (input)

```rust
let check = RnclCheckBox::new(&window, "Accept terms");
check.on_toggled(|on| println!("active: {on}"));
check.set_checked(true);
```

| Property | Type | |
|---|---|---|
| `text` | Text | caption |
| `checked` | Bool | checked state |

---

## Notes

- Signals also fire on programmatic changes (`set_text`, `set_checked`).
- Several handlers per signal are allowed; they run in registration order.
- For functionality without a dedicated method yet, `component.widget()` exposes
  the full `ruf::Widget` API.
