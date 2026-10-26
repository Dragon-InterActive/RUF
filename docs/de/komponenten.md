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
| `set_style_sheet(css)` | Gestaltung per CSS-Teilmenge, z. B. `"color: red; font-weight: bold;"` – siehe [Gestaltung](#gestaltung) |
| `set_name(n)` / `name()` | eindeutiger Bezeichner (`objectName`) |
| `widget()` | das zugrunde liegende `ruf::Widget` |
| `properties()` / `property(n)` / `set_property(n, v)` | Eigenschaftsmodell |

**Basis-Eigenschaften** (jede Komponente): `name`, `x`, `y`, `width`, `height`,
`visible`, `enabled`, `tool_tip`, `style_sheet`.

**Eigenschaftstypen:** Bool, Int, Text und Liste (`PropertyValue::TextList`, z. B. für
Einträge und Seitentitel).

| Kategorie | Komponenten |
|---|---|
| Container | `RnclWindow`, `RnclPanel`, `RnclGroupBox`, `RnclTabView`, `RnclScrollView` |
| Standard | `RnclButton`, `RnclLabel` |
| Eingabe | `RnclTextField`, `RnclTextArea`, `RnclCheckBox`, `RnclRadioButton`, `RnclComboBox`, `RnclListBox`, `RnclSpinBox`, `RnclSlider` |
| Anzeige | `RnclProgressBar` |

Jede Komponente hat zwei Konstruktoren: `new(&eltern, …)` und
`with_parent(Option<&Widget>, …)` (für Registry-Fabriken und zusammengesetzte Komponenten).

---

## `RnclWindow` – Fenster

Nativ: GTK `GtkWindow` · Container · Kategorie *Container*

```rust
let window = RnclWindow::new("Titel");   // Standardgröße 640×480, noch unsichtbar
window.set_size(400, 300);
window.show();
window.close();                          // letztes Fenster zu → exec() endet

window.on_resized(|w, h| println!("{w}×{h}"));          // Kinder neu anordnen
window.add_shortcut("Ctrl+S", || println!("Speichern"));
window.on_close_requested(|| {                           // false = offen lassen
    ruf::dialog::confirm(None, "Beenden", "Wirklich schließen?")
});

// Menüleiste; Auswahl per Maus oder Kürzel meldet on_menu mit der Kennung.
// Kürzel gelten im ganzen Fenster und gehen dem fokussierten Widget vor
// (Strg+Enter in einem Textfeld löst den Eintrag aus, keinen Zeilenumbruch).
window.set_menu_bar(vec![(
    "Datei".into(),
    Menu::new().item("file.open", "Öffnen …", Some("Ctrl+O")).separator()
               .item("file.quit", "Beenden", Some("Ctrl+Q")),
)]);
window.on_menu(|id| println!("Menü {id}"));
label.widget().popup_menu(&Menu::new().item("copy", "Kopieren", None), 10, 10); // Kontextmenü

// Bildschirmposition (Fensteranordnung speichern/wiederherstellen).
window.set_screen_position(100, 80);     // vor show() vorgemerkt; false unter Wayland
let pos = window.screen_position();      // Some(Point) solange angezeigt
```

GTK 4 kann Fenster nicht selbst positionieren; RUF setzt die Position nativ
(X11, Win32, macOS). Unter Wayland verbietet das Protokoll das – dort liefert
`set_screen_position` `false`.

| Eigenschaft | Typ | |
|---|---|---|
| `title` | Text | Fenstertitel |

## `RnclPanel` – Container

Nativ: GTK `RufFixed` · Container · Kategorie *Container*

```rust
let panel = RnclPanel::new(&window);
panel.set_geometry(10, 10, 300, 100);
let ok = RnclButton::new(&panel, "OK");  // Position relativ zum Panel
```

## `RnclButton` – Schaltfläche

Nativ: GTK `GtkButton` · Kategorie *Standard*

```rust
let button = RnclButton::new(&window, "Speichern");
button.on_clicked(|| println!("gespeichert"));
button.click();                          // programmatisch klicken
```

| Eigenschaft | Typ | |
|---|---|---|
| `text` | Text | Beschriftung |

## `RnclLabel` – Text

Nativ: GTK `GtkLabel` · Kategorie *Standard*

```rust
let label = RnclLabel::new(&window, "Status: <b>bereit</b>"); // einfaches HTML¹
label.set_text("Fertig");
label.set_word_wrap(true);                  // lange Texte umbrechen
```

| Eigenschaft | Typ | |
|---|---|---|
| `text` | Text | angezeigter Text |
| `word_wrap` | Bool | Zeilenumbruch |

¹ `<b> <i> <u> <small> <br>` – unter GTK formatiert, unter Windows und macOS als
reiner Text (Tags entfernt, `<br>` wird zum Zeilenumbruch).

## `RnclTextField` – Eingabefeld

Nativ: GTK `GtkEntry` · Kategorie *Eingabe*

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

Nativ: GTK `GtkCheckButton` · Kategorie *Eingabe*

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

Nativ: GTK `GtkCheckButton` · Kategorie *Eingabe*

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

Nativ: GTK `GtkFrame` · Container · Kategorie *Container*

```rust
let group = RnclGroupBox::new(&window, "Adresse");
group.set_geometry(10, 10, 300, 120);
RnclTextField::new(&group).set_position(12, 28);   // Titel belegt oben ca. 20 px
```

| Eigenschaft | Typ | |
|---|---|---|
| `title` | Text | Titel im Rahmen |

## `RnclTextArea` – mehrzeiliger Text (Memo)

Nativ: GTK `GtkTextView` · Kategorie *Eingabe*

```rust
let notes = RnclTextArea::new(&window);          // Standardgröße 240×120
notes.set_text("Zeile 1");
notes.append_line("Zeile 2");
notes.on_text_changed(|t| println!("{} Zeichen", t.len()));
```

Als **Code-Editor** (Schrift mit fester Breite, Syntax-Hervorhebung über reguläre
Ausdrücke, markierte Fehlerzeile):

```rust
let code = RnclTextArea::new(&window);
code.set_monospace(true);
code.add_highlight_rule(r"^\s*\[.*\]", "#8e44ad", true);   // TOML-Abschnitte, fett
code.add_highlight_rule("#.*$", "gray", false);             // Kommentare
code.set_marked_line(Some(4), "#f5b7b1");                   // Zeile 5 hinterlegen
code.goto_line(4);
code.set_marked_line(None, "");                             // Markierung entfernen
```

Regeln gelten zeilenweise, spätere überschreiben frühere. Die Muster folgen der
Syntax der [`regex`-Crate](https://docs.rs/regex) – ohne Lookarounds. Ersatz: Hat
ein Muster eine **Gruppe 1**, wird nur diese eingefärbt, z. B. `^\s*(\w+)\s*=` färbt
nur den Schlüssel vor dem Gleichheitszeichen. `add_highlight_rule` liefert `false`
bei ungültigem Muster oder ungültiger Farbe.

| Eigenschaft | Typ | |
|---|---|---|
| `text` | Text | Inhalt |
| `placeholder` | Text | Hinweis bei leerem Feld (Windows/macOS: nur gespeichert, nicht angezeigt) |
| `read_only` | Bool | nur lesbar |
| `monospace` | Bool | Schrift mit fester Zeichenbreite |

## `RnclComboBox` – Auswahlliste

Nativ: GTK `GtkDropDown` (mit freier Eingabe: `GtkEntry` + Vorschlagsliste) · Kategorie *Eingabe*

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

Nativ: GTK `GtkListBox` · Kategorie *Eingabe*

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

Nativ: GTK `GtkSpinButton` · Kategorie *Eingabe* · Standardbereich 0…99

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

Nativ: GTK `GtkScale` · Kategorie *Eingabe* · Standardbereich 0…99, waagerecht

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

Nativ: GTK `GtkProgressBar` · Kategorie *Anzeige* · Standardbereich 0…100

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

Nativ: GTK `GtkNotebook` · Container (Seiten) · Kategorie *Container*

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

## `RnclScrollView` – Scrollbereich

Nativ: GTK `GtkScrolledWindow` · Container (Inhalt) · Kategorie *Container*

Für Inhalte, die größer als der sichtbare Bereich sind – z. B. eine zur Laufzeit
erzeugte Liste von Checkboxen. Kinder kommen in `content()`.

```rust
let scroll = RnclScrollView::new(&window);
scroll.set_geometry(10, 10, 300, 200);
for i in 0..30 {
    RnclCheckBox::new(scroll.content(), &format!("Option {i}")).set_position(8, 8 + i * 24);
}
scroll.set_content_size(0, 8 + 30 * 24);   // Breite 0 = folgt dem sichtbaren Bereich
```

| Eigenschaft | Typ | |
|---|---|---|
| `content_width` / `content_height` | Int | Mindestgröße des Inhalts |

---

## `RnclCanvas` – Zeichenfläche

Nativ: GTK `GtkDrawingArea` · Kategorie *Anzeige*

Malt eine Liste von Formen (`ruf::Shape`: Rechteck mit Füllung, Rahmen, Radius,
gestrichelt; Text; Linie) und meldet Maus und Tastatur – die Grundlage eigener
Ansichten wie der Designfläche von Freya TD.

```rust
let canvas = RnclCanvas::new(&window);
canvas.set_geometry(0, 0, 400, 300);
canvas.set_shapes(vec![
    Shape::Rect { rect: Rect::new(10, 10, 120, 40), fill: Some(Color::rgb(250, 250, 250)),
                  stroke: Some(Color::rgb(160, 160, 160)), radius: 4, dashed: false },
    Shape::Text { x: 20, y: 20, text: "OK".into(), color: Color::rgb(0, 0, 0), size: 13, bold: false },
]);
canvas.on_mouse(|e| println!("{:?} {} {} Taste {}", e.action, e.x, e.y, e.button));
canvas.on_key(|e| if e.key == ruf::Key::Delete { /* … */ });
```

Ein Klick gibt der Zeichenfläche den Tastaturfokus. `MouseAction`: `Pressed`,
`Released`, `Moved`, `DoubleClicked`; Taste 1 = links, 3 = rechts.
Für Tests: `canvas.widget().simulate_mouse(…)` / `simulate_key(…)`.

## `RnclCodeEditor` – Code-Editor

Nativ: GtkSourceView 5 · Kategorie *Eingabe*

Syntax-Hervorhebung, Zeilennummern, aktuelle Zeile, Einrückung mit Leerzeichen,
passende Klammern und Marken am Rand. Das Farbschema folgt Hell/Dunkel.

```rust
let editor = RnclCodeEditor::new(&window);   // Sprache: Rust
editor.set_text(&std::fs::read_to_string("src/main.rs")?);
editor.set_language("toml");                  // oder "json", "markdown", "" …
editor.set_line_marks(vec![
    LineMark::new(12, MarkKind::Error, "cannot find value `x`"),   // Tooltip an der Marke
]);
editor.goto_line(12);                         // ab 1
editor.set_read_only(true);
editor.on_text_changed(|text| println!("{} Zeichen", text.len()));

editor.set_cursor(3, 8);                      // Zeile ab 1, Spalte ab 0
editor.show_completions(&[Completion { label: "len".into(), detail: None, insert: "len".into() }]);
editor.on_mouse(|e| if e.modifiers.ctrl { /* Strg+Klick */ });
```

**Faltung:** Klammerblöcke (`{}`, `[]`, `()`) über mehr als zwei Zeilen lassen
sich mit dem Pfeil am Rand falten (Rust, JSON, C-ähnliche Sprachen). Der Zustand
wandert beim Bearbeiten mit; springt der Cursor in einen gefalteten Bereich,
klappt er auf. Marken verborgener Zeilen stehen an der Kopfzeile. Die Bereiche
berechnet `ruf::fold_regions(text)` ohne Oberfläche.

```rust
for fold in editor.folds() {                  // Fold { start, end, folded }
    println!("{}–{} gefaltet: {}", fold.start, fold.end, fold.folded);
}
editor.set_folded(7, true);                   // Bereich ab Zeile 7 falten
editor.set_all_folded(false);                 // alles aufklappen
```

## `RnclTreeView` – Baumansicht

Nativ: GTK `GtkListBox` mit eingerückten Zeilen · Kategorie *Eingabe*

```rust
let tree = RnclTreeView::new(&window);
tree.set_items(vec![
    TreeItem::new("main", "main").with_child(TreeItem::new("main.ok", "ok_button")),
    TreeItem::new("services", "services").with_child(TreeItem::new("services.pool", "db_pool")).collapsed(),
]);
tree.select(Some("main.ok"));
tree.on_selection_changed(|id| println!("ausgewählt: {id}"));
tree.on_item_activated(|id| println!("Doppelklick: {id}"));
tree.toggle("services");                  // auf-/zuklappen
// Farbe und Zusatz rechts (z. B. Git-Status):
// TreeItem::new("src/main.rs", "main.rs").with_color(Color::rgb(0xd1, 0x9a, 0x3c)).with_badge("M")
// Rechtsklick: der Knoten ist dann gewählt; die Wahl meldet `on_menu` des Fensters.
let t = tree.clone();
tree.on_context_menu(move |id, x, y| {
    let menu = Menu::new().item("node.rename", "Umbenennen …", None);
    t.widget().popup_menu(&menu, x, y);
});
```

Knoten werden über ihre Kennung angesprochen; die Auswahl bleibt beim Ersetzen
der Knoten erhalten, solange ihr Knoten sichtbar ist.

## Gestaltung

`set_style_sheet` (bzw. die Eigenschaft `style_sheet`) versteht auf allen Systemen
dieselbe CSS-Teilmenge:

| Angabe | Wirkung |
|---|---|
| `color: #c0392b` | Textfarbe |
| `background: #fff8e1` / `background-color: …` | Hintergrund |
| `font-weight: bold` | fett (`normal` = normal) |

Farben: `#rgb`, `#rrggbb` oder Namen wie `red`, `gray`, `darkblue`
(siehe `ruf::Color::NAMES`). Andere Angaben werden ignoriert – das System bestimmt
das übrige Aussehen.

```rust
status.set_style_sheet("color: #1e8449; font-weight: bold;");
```

## Dienste für dynamische Oberflächen

### Widgets zur Laufzeit erzeugen und entfernen

Kinder, die nach `show()` des Elternteils entstehen, sind sofort sichtbar.
`widget().delete_later()` entfernt ein Widget samt Kindern; alle Verweise darauf
werden danach "tot" (`is_alive() == false`), ohne dass etwas abstürzt.

```rust
let row = RnclCheckBox::new(scroll.content(), "neu");   // sofort sichtbar
row.widget().delete_later();                            // wieder entfernen
```

### `ruf::Timer` – zeitgesteuerte Callbacks

```rust
let timer = ruf::Timer::new();
timer.set_single_shot(true);
timer.on_timeout(|| println!("300 ms Ruhe"));
timer.start(300);              // erneutes start() setzt die Wartezeit zurück (Entprellen)
ruf::Timer::single_shot(1000, || println!("einmalig nach 1 s"));
```

### `ruf::GuiSender` – Ergebnisse aus Hintergrund-Threads

Widgets dürfen nur im GUI-Thread benutzt werden. Lange Arbeiten (Netzwerk, Dateien,
externe Programme) laufen in einem eigenen Thread und schicken das Ergebnis per
`GuiSender` zurück; der Handler läuft im GUI-Thread und darf Widgets anfassen.

```rust
let label = RnclLabel::new(&window, "lädt …");
let l = label.clone();
let sender = ruf::GuiSender::new(move |text: String| l.set_text(&text));
std::thread::spawn(move || {
    let antwort = "fertig".to_owned();     // z. B. eine HTTP-Antwort
    sender.send(antwort);
});
```

`GuiSender` ist `Clone + Send + Sync`; Panics im Handler werden abgefangen.

### `ruf::dialog` – Standarddialoge

```rust
ruf::dialog::information(Some(window.widget()), "Hinweis", "Gespeichert.");
ruf::dialog::warning(None, "Achtung", "Datei fehlt.");
ruf::dialog::error(None, "Fehler", "Speichern fehlgeschlagen.");
if ruf::dialog::confirm(None, "Frage", "Änderungen verwerfen?") { /* … */ }
// Texteingabe; der Vorschlag ist bis zum letzten Punkt ausgewählt (Dateinamen).
if let Some(name) = ruf::dialog::input(None, "Umbenennen", "Neuer Name:", "main.rs") { /* … */ }
let secret = ruf::dialog::password(None, "Anmeldung", "Passphrase für den SSH-Schlüssel:"); // verdeckt
```

Dateiauswahl (liefert `None` bei Abbruch):

```rust
use ruf::dialog::FileFilter;
let rust = [FileFilter::new("Rust", &["*.rs"])];
let file = ruf::dialog::open_file(Some(window.widget()), "Öffnen", &rust);
let target = ruf::dialog::save_file(None, "Speichern unter", "main.rs", &rust);
let folder = ruf::dialog::choose_folder(None, "Projektordner");
```

Die Dialoge sind modal und blockieren bis zur Antwort – in automatischen Tests
nicht aufrufen.

---

## Hinweise

- Signale feuern auch bei programmatischen Änderungen (`set_text`, `set_checked`).
- Mehrere Handler pro Signal sind erlaubt; sie laufen in Registrierungsreihenfolge.
- Für Funktionen, die (noch) keine eigene Methode haben, steht `komponente.widget()`
  mit der vollständigen `ruf::Widget`-API bereit.
