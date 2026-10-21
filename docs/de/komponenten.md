# Komponenten-Referenz

Alle Komponenten implementieren `ruf::Component` und besitzen damit:

| Methode | Beschreibung |
|---|---|
| `set_position(x, y)` / `position()` | Position relativ zum Elternteil |
| `set_size(w, h)` / `size()` | Größe in Pixeln |
| `set_geometry(x, y, w, h)` / `geometry()` | beides zusammen |
| `show()` / `hide()` / `is_visible()` | Sichtbarkeit |
| `set_enabled(b)` / `is_enabled()` | Bedienbarkeit |
| `set_tool_tip(text)` | Hinweistext |
| `set_style_sheet(css)` | Qt-Stylesheet, z. B. `"color: red; font-weight: bold;"` |
| `set_name(n)` / `name()` | eindeutiger Bezeichner (`objectName`) |
| `widget()` | das zugrunde liegende `ruf::Widget` |
| `properties()` / `property(n)` / `set_property(n, v)` | Eigenschaftsmodell |

**Basis-Eigenschaften** (jede Komponente): `name`, `x`, `y`, `width`, `height`,
`visible`, `enabled`, `tool_tip`, `style_sheet`.

Jede Komponente hat zwei Konstruktoren: `new(&eltern, …)` und
`with_parent(Option<&Widget>, …)` (für Registry-Fabriken und zusammengesetzte Komponenten).

---

## `RnclWindow` – Fenster

Qt: `QWidget` (Top-Level) · Container · Kategorie *Container*

```rust
let window = RnclWindow::new("Titel");   // Standardgröße 640×480, noch unsichtbar
window.set_size(400, 300);
window.show();
window.close();                          // letztes Fenster zu → exec() endet
```

| Eigenschaft | Typ | |
|---|---|---|
| `title` | Text | Fenstertitel |

## `RnclPanel` – Container

Qt: `QWidget` · Container · Kategorie *Container*

```rust
let panel = RnclPanel::new(&window);
panel.set_geometry(10, 10, 300, 100);
let ok = RnclButton::new(&panel, "OK");  // Position relativ zum Panel
```

## `RnclButton` – Schaltfläche

Qt: `QPushButton` · Kategorie *Standard*

```rust
let button = RnclButton::new(&window, "Speichern");
button.on_clicked(|| println!("gespeichert"));
button.click();                          // programmatisch klicken
```

| Eigenschaft | Typ | |
|---|---|---|
| `text` | Text | Beschriftung |

## `RnclLabel` – Text

Qt: `QLabel` · Kategorie *Standard*

```rust
let label = RnclLabel::new(&window, "Status: <b>bereit</b>"); // einfaches HTML
label.set_text("Fertig");
```

| Eigenschaft | Typ | |
|---|---|---|
| `text` | Text | angezeigter Text |

## `RnclTextField` – Eingabefeld

Qt: `QLineEdit` · Kategorie *Eingabe*

```rust
let field = RnclTextField::new(&window);
field.set_placeholder("E-Mail");
field.on_text_changed(|t| println!("neu: {t}"));
field.on_return_pressed(|| println!("Enter"));

let pass = RnclTextField::new(&window);
pass.set_password_mode(true);
```

| Eigenschaft | Typ | |
|---|---|---|
| `text` | Text | eingegebener Text |
| `placeholder` | Text | Hinweis bei leerem Feld |
| `password` | Bool | Eingabe verdecken |

## `RnclCheckBox` – Kontrollkästchen

Qt: `QCheckBox` · Kategorie *Eingabe*

```rust
let check = RnclCheckBox::new(&window, "AGB akzeptiert");
check.on_toggled(|an| println!("aktiv: {an}"));
check.set_checked(true);
```

| Eigenschaft | Typ | |
|---|---|---|
| `text` | Text | Beschriftung |
| `checked` | Bool | Häkchen gesetzt |

---

## Hinweise

- Signale feuern auch bei programmatischen Änderungen (`set_text`, `set_checked`).
- Mehrere Handler pro Signal sind erlaubt; sie laufen in Registrierungsreihenfolge.
- Für Funktionen, die (noch) keine eigene Methode haben, steht `komponente.widget()`
  mit der vollständigen `ruf::Widget`-API bereit.
