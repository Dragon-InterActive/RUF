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

**Property types:** Bool, Int, Text and List (`PropertyValue::TextList`, e.g. for
entries and page titles).

| Category | Components |
|---|---|
| Container | `RnclWindow`, `RnclPanel`, `RnclGroupBox`, `RnclTabView` |
| Standard | `RnclButton`, `RnclLabel` |
| Eingabe (input) | `RnclTextField`, `RnclTextArea`, `RnclCheckBox`, `RnclRadioButton`, `RnclComboBox`, `RnclListBox`, `RnclSpinBox`, `RnclSlider` |
| Anzeige (display) | `RnclProgressBar` |

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
| `read_only` | Bool | read-only |

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

## `RnclRadioButton` – radio button

Qt: `QRadioButton` · category *Eingabe* (input)

Radio buttons with the same parent are mutually exclusive – for several groups use
one `RnclGroupBox` (or `RnclPanel`) per group.

```rust
let group = RnclGroupBox::new(&window, "Payment");
let card = RnclRadioButton::new(&group, "Card");
card.set_position(12, 26);
let cash = RnclRadioButton::new(&group, "Cash");
cash.set_position(12, 52);
card.set_checked(true);
cash.on_toggled(|on| println!("cash: {on}"));
```

| Property | Type | |
|---|---|---|
| `text` | Text | caption |
| `checked` | Bool | selected |

## `RnclGroupBox` – framed group

Qt: `QGroupBox` · container · category *Container*

```rust
let group = RnclGroupBox::new(&window, "Address");
group.set_geometry(10, 10, 300, 120);
RnclTextField::new(&group).set_position(12, 28);   // the title takes ~20 px at the top
```

| Property | Type | |
|---|---|---|
| `title` | Text | frame title |

## `RnclTextArea` – multi-line text (memo)

Qt: `QPlainTextEdit` · category *Eingabe* (input)

```rust
let notes = RnclTextArea::new(&window);          // default size 240×120
notes.set_text("line 1");
notes.append_line("line 2");
notes.on_text_changed(|t| println!("{} chars", t.len()));
```

| Property | Type | |
|---|---|---|
| `text` | Text | content |
| `placeholder` | Text | hint while empty |
| `read_only` | Bool | read-only |

## `RnclComboBox` – drop-down list

Qt: `QComboBox` · category *Eingabe* (input)

```rust
let city = RnclComboBox::new(&window);
city.set_items(&["Berlin", "Hamburg", "Munich"]); // first item gets selected
city.set_current_text("Hamburg");
city.on_current_index_changed(|i| println!("index {i}"));
city.set_editable(true);                          // allow free text as well
```

| Property | Type | |
|---|---|---|
| `items` | List | entries |
| `current_index` | Int | selected entry (`-1` = none) |
| `text` | Text | current text |
| `editable` | Bool | allow free text |

## `RnclListBox` – list

Qt: `QListWidget` · category *Eingabe* (input)

```rust
let list = RnclListBox::new(&window);             // default size 160×120
list.set_items(&["Red", "Green", "Blue"]);
list.on_current_index_changed(|i| println!("row {i}"));
if let Some(t) = list.current_text() { println!("{t}"); }
```

| Property | Type | |
|---|---|---|
| `items` | List | entries |
| `current_index` | Int | selected row (`-1` = none) |

## `RnclSpinBox` – number field

Qt: `QSpinBox` · category *Eingabe* (input) · default range 0…99

```rust
let age = RnclSpinBox::new(&window);
age.set_range(0, 120);
age.set_value(30);
age.on_value_changed(|v| println!("{v}"));
```

| Property | Type | |
|---|---|---|
| `value` / `minimum` / `maximum` | Int | value and range |
| `read_only` | Bool | read-only |

## `RnclSlider` – slider

Qt: `QSlider` · category *Eingabe* (input) · default range 0…99, horizontal

```rust
let volume = RnclSlider::new(&window);
volume.set_range(0, 100);
volume.on_value_changed(|v| println!("{v} %"));
volume.set_vertical(true);                        // swap width/height if needed
```

| Property | Type | |
|---|---|---|
| `value` / `minimum` / `maximum` | Int | value and range |
| `vertical` | Bool | vertical orientation |

## `RnclProgressBar` – progress bar

Qt: `QProgressBar` · category *Anzeige* (display) · default range 0…100

```rust
let bar = RnclProgressBar::new(&window);
bar.set_value(75);
bar.set_busy(true);                               // "busy" without a percentage
```

| Property | Type | |
|---|---|---|
| `value` / `minimum` / `maximum` | Int | value and range |
| `vertical` | Bool | vertical orientation |

## `RnclTabView` – tabs

Qt: `QTabWidget` · container (pages) · category *Container*

```rust
let tabs = RnclTabView::new(&window);
tabs.set_geometry(10, 10, 400, 300);
let general = tabs.add_tab("General");            // every page is an RnclPanel
RnclCheckBox::new(&general, "Autostart").set_position(10, 10);
let expert = tabs.add_tab("Advanced");
tabs.on_current_index_changed(|i| println!("page {i}"));
tabs.remove_tab(1);                               // removes the page and its content
```

| Property | Type | |
|---|---|---|
| `tabs` | List | page titles (setting renames, adds or removes pages) |
| `current_index` | Int | visible page |

---

## Notes

- Signals also fire on programmatic changes (`set_text`, `set_checked`).
- Several handlers per signal are allowed; they run in registration order.
- For functionality without a dedicated method yet, `component.widget()` exposes
  the full `ruf::Widget` API.
