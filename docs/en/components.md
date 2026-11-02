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
| `set_style_sheet(css)` | styling via a CSS subset, e.g. `"color: red; font-weight: bold;"` – see [Styling](#styling) |
| `set_name(n)` / `name()` | unique identifier (`objectName`) |
| `widget()` | the underlying `ruf::Widget` |
| `properties()` / `property(n)` / `set_property(n, v)` | property model |

**Base properties** (every component): `name`, `x`, `y`, `width`, `height`,
`visible`, `enabled`, `tool_tip`, `style_sheet`.

**Property types:** Bool, Int, Text and List (`PropertyValue::TextList`, e.g. for
entries and page titles).

| Category | Components |
|---|---|
| Container | `RnclWindow`, `RnclPanel`, `RnclGroupBox`, `RnclTabView`, `RnclScrollView`, `RnclSplitter` |
| Standard | `RnclButton`, `RnclLabel` |
| Eingabe (input) | `RnclTextField`, `RnclTextArea`, `RnclCheckBox`, `RnclRadioButton`, `RnclComboBox`, `RnclListBox`, `RnclSpinBox`, `RnclSlider` |
| Anzeige (display) | `RnclProgressBar` |

Every component has two constructors: `new(&parent, …)` and
`with_parent(Option<&Widget>, …)` (for registry factories and composite components).

---

## `RnclWindow` – window

Native: GTK `GtkWindow` · container · category *Container*

```rust
let window = RnclWindow::new("Title");   // default size 640×480, initially hidden
window.set_size(400, 300);
window.show();
window.close();                          // last window closed → exec() returns

window.on_resized(|w, h| println!("{w}×{h}"));          // re-arrange children
window.add_shortcut("Ctrl+S", || println!("save"));
window.on_close_requested(|| {                           // false = keep open
    ruf::dialog::confirm(None, "Quit", "Really close?")
});

// Menu bar; choosing by mouse or shortcut reports on_menu with the id.
// Shortcuts work in the whole window and take precedence over the focused
// widget (Ctrl+Enter in a text area triggers the item, not a line break).
window.set_menu_bar(vec![(
    "File".into(),
    Menu::new().item("file.open", "Open …", Some("Ctrl+O")).separator()
               .item("file.quit", "Quit", Some("Ctrl+Q")),
)]);
window.on_menu(|id| println!("menu {id}"));
// Several windows: the main menu's shortcuts work here too (own ones win).
other_window.inherit_menu_shortcuts(&window);
label.widget().popup_menu(&Menu::new().item("copy", "Copy", None), 10, 10); // context menu

// Screen position (save/restore the window arrangement).
window.set_screen_position(100, 80);     // remembered before show(); false on Wayland
let pos = window.screen_position();      // Some(Point) while shown
```

GTK 4 cannot position windows itself; RUF sets the position natively (X11, Win32,
macOS). The Wayland protocol forbids it – there `set_screen_position` returns `false`.

| Property | Type | |
|---|---|---|
| `title` | Text | window title |

## `RnclPanel` – container

Native: GTK `RufFixed` · container · category *Container*

```rust
let panel = RnclPanel::new(&window);
panel.set_geometry(10, 10, 300, 100);
let ok = RnclButton::new(&panel, "OK");  // position relative to the panel
```

## `RnclButton` – push button

Native: GTK `GtkButton` · category *Standard*

```rust
let button = RnclButton::new(&window, "Save");
button.on_clicked(|| println!("saved"));
button.click();                          // click programmatically
```

| Property | Type | |
|---|---|---|
| `text` | Text | caption |

## `RnclLabel` – text

Native: GTK `GtkLabel` · category *Standard*

```rust
let label = RnclLabel::new(&window, "Status: <b>ready</b>"); // simple HTML¹
label.set_text("Done");
label.set_word_wrap(true);                  // wrap long texts
```

| Property | Type | |
|---|---|---|
| `text` | Text | displayed text |
| `word_wrap` | Bool | word wrap |

¹ `<b> <i> <u> <small> <br>` – formatted on GTK, plain text on Windows and macOS
(tags removed, `<br>` becomes a line break).

## `RnclTextField` – line edit

Native: GTK `GtkEntry` · category *Eingabe* (input)

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

Native: GTK `GtkCheckButton` · category *Eingabe* (input)

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

Native: GTK `GtkCheckButton` · category *Eingabe* (input)

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

Native: GTK `GtkFrame` · container · category *Container*

```rust
let group = RnclGroupBox::new(&window, "Address");
group.set_geometry(10, 10, 300, 120);
RnclTextField::new(&group).set_position(12, 28);   // the title takes ~20 px at the top
```

| Property | Type | |
|---|---|---|
| `title` | Text | frame title |

## `RnclTextArea` – multi-line text (memo)

Native: GTK `GtkTextView` · category *Eingabe* (input)

```rust
let notes = RnclTextArea::new(&window);          // default size 240×120
notes.set_text("line 1");
notes.append_line("line 2");
notes.on_text_changed(|t| println!("{} chars", t.len()));
notes.set_word_wrap(true);                      // wrap long lines
let (needed, line) = notes.widget().text_height(240).unwrap(); // height with wrapping, line height
```

As a **code editor** (fixed-width font, syntax highlighting via regular expressions,
marked error line):

```rust
let code = RnclTextArea::new(&window);
code.set_monospace(true);
code.add_highlight_rule(r"^\s*\[.*\]", "#8e44ad", true);   // TOML sections, bold
code.add_highlight_rule("#.*$", "gray", false);             // comments
code.set_marked_line(Some(4), "#f5b7b1");                   // highlight line 5
code.goto_line(4);
code.set_marked_line(None, "");                             // remove the mark
```

Rules apply per line; later rules override earlier ones. Patterns follow the syntax
of the [`regex` crate](https://docs.rs/regex) – without lookarounds. Replacement: if
a pattern has a **group 1**, only that group is coloured, e.g. `^\s*(\w+)\s*=` colours
only the key before the equals sign. `add_highlight_rule` returns `false` for an
invalid pattern or colour.

| Property | Type | |
|---|---|---|
| `text` | Text | content |
| `placeholder` | Text | hint while empty (Windows/macOS: stored only, not shown) |
| `read_only` | Bool | read-only |
| `monospace` | Bool | fixed-width font |

## `RnclComboBox` – drop-down list

Native: GTK `GtkDropDown` (editable: `GtkEntry` + suggestion list) · category *Eingabe* (input)

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

Native: GTK `GtkListBox` · category *Eingabe* (input)

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

Native: GTK `GtkSpinButton` · category *Eingabe* (input) · default range 0…99

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

Native: GTK `GtkScale` · category *Eingabe* (input) · default range 0…99, horizontal

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

Native: GTK `GtkProgressBar` · category *Anzeige* (display) · default range 0…100

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

Native: GTK `GtkNotebook` · container (pages) · category *Container*

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

## `RnclScrollView` – scroll area

Native: GTK `GtkScrolledWindow` · container (content) · category *Container*

For content larger than the visible area – e.g. a list of check boxes created at
runtime. Children go into `content()`.

```rust
let scroll = RnclScrollView::new(&window);
scroll.set_geometry(10, 10, 300, 200);
for i in 0..30 {
    RnclCheckBox::new(scroll.content(), &format!("Option {i}")).set_position(8, 8 + i * 24);
}
scroll.set_content_size(0, 8 + 30 * 24);
scroll.scroll_to(0, 20 * 24);              // bring option 20 to the top   // width 0 = follow the visible area
```

| Property | Type | |
|---|---|---|
| `content_width` / `content_height` | Int | minimum content size |

---

## `RnclCanvas` – canvas

Native: GTK `GtkDrawingArea` · category *Display*

Paints a list of shapes (`ruf::Shape`: rectangle with fill, border, radius, dashed;
text; line) and reports mouse and keyboard – the basis of custom views such as the
design surface of Freya TD.

```rust
let canvas = RnclCanvas::new(&window);
canvas.set_geometry(0, 0, 400, 300);
canvas.set_shapes(vec![
    Shape::Rect { rect: Rect::new(10, 10, 120, 40), fill: Some(Color::rgb(250, 250, 250)),
                  stroke: Some(Color::rgb(160, 160, 160)), radius: 4, dashed: false },
    Shape::Text { x: 20, y: 20, text: "OK".into(), color: Color::rgb(0, 0, 0), size: 13, bold: false },
]);
canvas.on_mouse(|e| println!("{:?} {} {} button {}", e.action, e.x, e.y, e.button));
canvas.on_key(|e| if e.key == ruf::Key::Delete { /* … */ });
```

A click (or `canvas.widget().focus()`) gives the canvas keyboard focus.
`MouseAction`: `Pressed`, `Released`, `Moved`, `DoubleClicked`, `ScrollUp`,
`ScrollDown` (wheel); button 1 = left, 3 = right. `KeyEvent::text` is the
character as typed (`A`, `ä`, `@`), `key` the key. For text input with dead keys
(`^` + `e` = `ê`), accents and input methods (IME): `canvas.on_text_input(|text| …)` –
characters without Ctrl/Alt then arrive there, arrows, Enter, Ctrl+C etc. still via
`on_key`; `widget().set_input_cursor(rect)` places candidate windows.

Monospace text (e.g. a terminal) with `Shape::MonoText` (background, bold,
italic, underline); the cell size comes from `ruf::measure_text("M", 13, false, true)`.
Clipboard: `ruf::clipboard::set_text("…")` and `ruf::clipboard::read_text(|t| …)`
(asynchronous). For tests:
`canvas.widget().simulate_mouse(…)` / `simulate_key(…)`.

## `RnclCodeEditor` – code editor

Native: GtkSourceView 5 · category *Input*

Syntax highlighting, line numbers, current line, indentation with spaces, matching
brackets and marks in the gutter. The color scheme follows light/dark.

```rust
let editor = RnclCodeEditor::new(&window);   // language: Rust
editor.set_text(&std::fs::read_to_string("src/main.rs")?);
editor.set_language("toml");                  // or "json", "markdown", "" …
editor.set_line_marks(vec![
    LineMark::new(12, MarkKind::Error, "cannot find value `x`"),   // tooltip on the mark
]);
editor.goto_line(12);                         // 1-based
editor.set_read_only(true);
editor.on_text_changed(|text| println!("{} chars", text.len()));

editor.set_cursor(3, 8);                      // line 1-based, column 0-based
editor.show_completions(&[Completion { label: "len".into(), detail: None, insert: "len".into() }]);
editor.on_mouse(|e| if e.modifiers.ctrl { /* Ctrl+Click */ });
```

**Folding:** bracket blocks (`{}`, `[]`, `()`) spanning more than two lines fold
with the arrow in the gutter (Rust, JSON, C-like languages). The state follows
edits; moving the cursor into a folded block unfolds it. Marks of hidden lines
appear on the header line. `ruf::fold_regions(text)` computes the blocks without UI.

```rust
for fold in editor.folds() {                  // Fold { start, end, folded }
    println!("{}–{} folded: {}", fold.start, fold.end, fold.folded);
}
editor.set_folded(7, true);                   // fold the block starting on line 7
editor.set_all_folded(false);                 // unfold everything
// Change bars (e.g. Git): green new, blue modified, red triangle = lines removed
editor.set_diff_marks(vec![DiffMark { kind: DiffKind::Added, line: 3, count: 2 }]);
```

**Debugging:** the leftmost column holds breakpoints (red dot, orange =
conditional, red diamond = logpoint, gray = disabled, gray circle = unresolved)
and the yellow arrow of the line the debugger stands on; that line is
highlighted. A click into the gutter reports line and mouse button.

```rust
editor.set_breakpoint_marks(vec![BreakpointMark { line: 8, kind: BreakpointKind::Normal }]);
editor.set_execution_line(Some(8));           // None removes the arrow
editor.on_gutter_clicked(|line, button| println!("line {line}, button {button}"));
```

## `RnclSplitter` – movable divider

Native: canvas with a resize mouse cursor · category *Container*

The splitter lays out nothing itself: while dragged it reports the offset in
pixels; the application resizes its areas (and moves the splitter).

```rust
let splitter = RnclSplitter::new(&window, true);   // horizontal: top/bottom
let split = Rc::new(Cell::new(200));               // height of the top area
splitter.on_dragged(move |delta| split.set(split.get() + delta)); // then lay out again
```

## `RnclTreeView` – tree view

Native: GTK `GtkListBox` with indented rows · category *Input*

```rust
let tree = RnclTreeView::new(&window);
tree.set_items(vec![
    TreeItem::new("main", "main").with_child(TreeItem::new("main.ok", "ok_button")),
    TreeItem::new("services", "services").with_child(TreeItem::new("services.pool", "db_pool")).collapsed(),
]);
tree.select(Some("main.ok"));
tree.on_selection_changed(|id| println!("selected: {id}"));
tree.on_item_activated(|id| println!("double click: {id}"));
tree.toggle("services");                  // expand/collapse
tree.on_item_toggled(|id, open| { /* e.g. load children only now */ });
// Color and a badge on the right (e.g. a Git status):
// TreeItem::new("src/main.rs", "main.rs").with_color(Color::rgb(0xd1, 0x9a, 0x3c)).with_badge("M")
// Right click: the node is selected then; the choice reaches the window's `on_menu`.
let t = tree.clone();
tree.on_context_menu(move |id, x, y| {
    let menu = Menu::new().item("node.rename", "Rename …", None);
    t.widget().popup_menu(&menu, x, y);
});
```

Nodes are addressed by their id; the selection survives replacing the nodes as long
as its node stays visible.

## Styling

`set_style_sheet` (or the property `style_sheet`) understands the same CSS subset on
every system:

| Declaration | Effect |
|---|---|
| `color: #c0392b` | text colour |
| `background: #fff8e1` / `background-color: …` | background |
| `font-weight: bold` | bold (`normal` = regular) |

Colours: `#rgb`, `#rrggbb` or names such as `red`, `gray`, `darkblue`
(see `ruf::Color::NAMES`). Other declarations are ignored – the system determines
the rest of the appearance.

```rust
status.set_style_sheet("color: #1e8449; font-weight: bold;");
```

## Services for dynamic user interfaces

### Creating and removing widgets at runtime

Children created after the parent's `show()` are visible immediately.
`widget().delete_later()` removes a widget including its children; all references
to it become "dead" (`is_alive() == false`) without crashing.

```rust
let row = RnclCheckBox::new(scroll.content(), "new");   // visible right away
row.widget().delete_later();                            // remove it again
```

### `ruf::Timer` – timed callbacks

```rust
let timer = ruf::Timer::new();
timer.set_single_shot(true);
timer.on_timeout(|| println!("300 ms of silence"));
timer.start(300);              // calling start() again restarts the wait (debouncing)
ruf::Timer::single_shot(1000, || println!("once after 1 s"));
```

### `ruf::GuiSender` – results from background threads

Widgets may only be used on the GUI thread. Long-running work (network, files,
external programs) runs on its own thread and sends the result back through a
`GuiSender`; the handler runs on the GUI thread and may touch widgets.

```rust
let label = RnclLabel::new(&window, "loading …");
let l = label.clone();
let sender = ruf::GuiSender::new(move |text: String| l.set_text(&text));
std::thread::spawn(move || {
    let answer = "done".to_owned();        // e.g. an HTTP response
    sender.send(answer);
});
```

`GuiSender` is `Clone + Send + Sync`; panics in the handler are caught.

### `ruf::dialog` – standard dialogs

```rust
ruf::dialog::information(Some(window.widget()), "Note", "Saved.");
ruf::dialog::warning(None, "Attention", "File missing.");
ruf::dialog::error(None, "Error", "Saving failed.");
if ruf::dialog::confirm(None, "Question", "Discard changes?") { /* … */ }
// Text input; the suggestion is selected up to the last dot (file names).
if let Some(name) = ruf::dialog::input(None, "Rename", "New name:", "main.rs") { /* … */ }
let secret = ruf::dialog::password(None, "Sign in", "Passphrase for the SSH key:"); // hidden
```

File chooser (returns `None` when cancelled):

```rust
use ruf::dialog::FileFilter;
let rust = [FileFilter::new("Rust", &["*.rs"])];
let file = ruf::dialog::open_file(Some(window.widget()), "Open", &rust);
let target = ruf::dialog::save_file(None, "Save as", "main.rs", &rust);
let folder = ruf::dialog::choose_folder(None, "Project folder");
```

The dialogs are modal and block until answered – do not call them in automated
tests.

---

## Notes

- Signals also fire on programmatic changes (`set_text`, `set_checked`).
- Several handlers per signal are allowed; they run in registration order.
- For functionality without a dedicated method yet, `component.widget()` exposes
  the full `ruf::Widget` API.
