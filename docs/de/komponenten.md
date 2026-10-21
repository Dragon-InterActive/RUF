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

**Eigenschaftstypen:** Bool, Int, Text und Liste (`PropertyValue::TextList`, z. B. für
Einträge und Seitentitel).

| Kategorie | Komponenten |
|---|---|
| Container | `RnclWindow`, `RnclPanel`, `RnclGroupBox`, `RnclTabView` |
| Standard | `RnclButton`, `RnclLabel` |
| Eingabe | `RnclTextField`, `RnclTextArea`, `RnclCheckBox`, `RnclRadioButton`, `RnclComboBox`, `RnclListBox`, `RnclSpinBox`, `RnclSlider` |
| Anzeige | `RnclProgressBar` |

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
| `read_only` | Bool | nur lesbar |

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

## `RnclRadioButton` – Optionsfeld

Qt: `QRadioButton` · Kategorie *Eingabe*

Optionsfelder mit demselben Elternteil schließen sich gegenseitig aus – für mehrere
Gruppen je Gruppe eine `RnclGroupBox` (oder ein `RnclPanel`) verwenden.

```rust
let group = RnclGroupBox::new(&window, "Zahlungsart");
let card = RnclRadioButton::new(&group, "Karte");
card.set_position(12, 26);
let cash = RnclRadioButton::new(&group, "Bar");
cash.set_position(12, 52);
card.set_checked(true);
cash.on_toggled(|an| println!("Bar: {an}"));
```

| Eigenschaft | Typ | |
|---|---|---|
| `text` | Text | Beschriftung |
| `checked` | Bool | ausgewählt |

## `RnclGroupBox` – Gruppe mit Rahmen

Qt: `QGroupBox` · Container · Kategorie *Container*

```rust
let group = RnclGroupBox::new(&window, "Adresse");
group.set_geometry(10, 10, 300, 120);
RnclTextField::new(&group).set_position(12, 28);   // Titel belegt oben ca. 20 px
```

| Eigenschaft | Typ | |
|---|---|---|
| `title` | Text | Titel im Rahmen |

## `RnclTextArea` – mehrzeiliger Text (Memo)

Qt: `QPlainTextEdit` · Kategorie *Eingabe*

```rust
let notes = RnclTextArea::new(&window);          // Standardgröße 240×120
notes.set_text("Zeile 1");
notes.append_line("Zeile 2");
notes.on_text_changed(|t| println!("{} Zeichen", t.len()));
```

| Eigenschaft | Typ | |
|---|---|---|
| `text` | Text | Inhalt |
| `placeholder` | Text | Hinweis bei leerem Feld |
| `read_only` | Bool | nur lesbar |

## `RnclComboBox` – Auswahlliste

Qt: `QComboBox` · Kategorie *Eingabe*

```rust
let city = RnclComboBox::new(&window);
city.set_items(&["Berlin", "Hamburg", "München"]); // erster Eintrag wird gewählt
city.set_current_text("Hamburg");
city.on_current_index_changed(|i| println!("Index {i}"));
city.set_editable(true);                           // zusätzlich freie Eingabe
```

| Eigenschaft | Typ | |
|---|---|---|
| `items` | Liste | Einträge |
| `current_index` | Int | ausgewählter Eintrag (`-1` = keiner) |
| `text` | Text | aktueller Text |
| `editable` | Bool | freie Eingabe erlauben |

## `RnclListBox` – Liste

Qt: `QListWidget` · Kategorie *Eingabe*

```rust
let list = RnclListBox::new(&window);              // Standardgröße 160×120
list.set_items(&["Rot", "Grün", "Blau"]);
list.on_current_index_changed(|i| println!("Zeile {i}"));
if let Some(t) = list.current_text() { println!("{t}"); }
```

| Eigenschaft | Typ | |
|---|---|---|
| `items` | Liste | Einträge |
| `current_index` | Int | ausgewählte Zeile (`-1` = keine) |

## `RnclSpinBox` – Zahlenfeld

Qt: `QSpinBox` · Kategorie *Eingabe* · Standardbereich 0…99

```rust
let age = RnclSpinBox::new(&window);
age.set_range(0, 120);
age.set_value(30);
age.on_value_changed(|v| println!("{v}"));
```

| Eigenschaft | Typ | |
|---|---|---|
| `value` / `minimum` / `maximum` | Int | Wert und Bereich |
| `read_only` | Bool | nur lesbar |

## `RnclSlider` – Schieberegler

Qt: `QSlider` · Kategorie *Eingabe* · Standardbereich 0…99, waagerecht

```rust
let volume = RnclSlider::new(&window);
volume.set_range(0, 100);
volume.on_value_changed(|v| println!("{v} %"));
volume.set_vertical(true);                         // Breite/Höhe ggf. tauschen
```

| Eigenschaft | Typ | |
|---|---|---|
| `value` / `minimum` / `maximum` | Int | Wert und Bereich |
| `vertical` | Bool | senkrechte Ausrichtung |

## `RnclProgressBar` – Fortschrittsbalken

Qt: `QProgressBar` · Kategorie *Anzeige* · Standardbereich 0…100

```rust
let bar = RnclProgressBar::new(&window);
bar.set_value(75);
bar.set_busy(true);                                // "beschäftigt" ohne Prozentwert
```

| Eigenschaft | Typ | |
|---|---|---|
| `value` / `minimum` / `maximum` | Int | Wert und Bereich |
| `vertical` | Bool | senkrechte Ausrichtung |

## `RnclTabView` – Registerkarten

Qt: `QTabWidget` · Container (Seiten) · Kategorie *Container*

```rust
let tabs = RnclTabView::new(&window);
tabs.set_geometry(10, 10, 400, 300);
let general = tabs.add_tab("Allgemein");           // jede Seite ist ein RnclPanel
RnclCheckBox::new(&general, "Autostart").set_position(10, 10);
let expert = tabs.add_tab("Erweitert");
tabs.on_current_index_changed(|i| println!("Seite {i}"));
tabs.remove_tab(1);                                // entfernt Seite samt Inhalt
```

| Eigenschaft | Typ | |
|---|---|---|
| `tabs` | Liste | Seitentitel (Setzen benennt um, ergänzt bzw. entfernt Seiten) |
| `current_index` | Int | sichtbare Seite |

---

## Hinweise

- Signale feuern auch bei programmatischen Änderungen (`set_text`, `set_checked`).
- Mehrere Handler pro Signal sind erlaubt; sie laufen in Registrierungsreihenfolge.
- Für Funktionen, die (noch) keine eigene Methode haben, steht `komponente.widget()`
  mit der vollständigen `ruf::Widget`-API bereit.
