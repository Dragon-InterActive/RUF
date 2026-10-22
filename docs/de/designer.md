# Designer-Anbindung (Freya TD)

Der Form-Designer spricht **ausschließlich** drei Abstraktionen an – nie konkrete
Rust-Typen. Dadurch erscheinen Komponenten von Drittentwicklern automatisch in der
Palette.

| Abstraktion | Aufgabe |
|---|---|
| `ComponentRegistry` | Palette: welche Komponenten gibt es, in welcher Kategorie? Erzeugen per Typname. |
| `Component` (als `Box<dyn Component>`) | die platzierte Instanz |
| `PropertyDescriptor` / `PropertyValue` | Eigenschaftsinspektor: auflisten, lesen, schreiben |

## Palette aufbauen

```rust
let registry = rncl::registry();
for category in registry.categories() {
    println!("[{category}]");
    for info in registry.iter().filter(|i| i.category == category) {
        println!("  {} – {}", info.type_name, info.description);
    }
}
```

## Komponente platzieren

```rust
let form = registry.create("RnclWindow", None)?;
let mut button = registry.create("RnclButton", Some(form.widget()))?;
button.set_name("btnOk");
button.set_property("text", "OK".into())?;
button.set_property("x", 20.into())?;
```

## Eigenschaftsinspektor

```rust
for d in button.properties() {
    let value = button.property(d.name);
    println!("{:<12} {:<5} {:?}  // {}", d.name, d.ty, value, d.description);
}
```

`set_property` liefert bei Fehlern ein `PropertyError`:

| Fehler | Bedeutung |
|---|---|
| `Unknown { name }` | Komponente kennt die Eigenschaft nicht |
| `TypeMismatch { name, expected, found }` | falscher Werttyp |
| `ReadOnly { name }` | Eigenschaft ist nur lesbar |

Werttypen (`PropertyType`): `Bool`, `Int`, `Text` und `TextList` (z. B. `items` einer
Combobox oder `tabs` einer Registerkarte):

```rust
combo.set_property("items", ["Berlin", "Hamburg"].into())?;
```

## Formular aus einer Beschreibung laden

`crates/rncl/examples/designer.rs` baut ein Formular aus einer Textbeschreibung auf –
ein Vorgeschmack auf das spätere Freya-TD-Dateiformat:

```text
RnclWindow    frmMain   title="Designer-Demo" width=360 height=200
RnclLabel     lblName   text="Name:" x=20 y=24
RnclTextField txtName   x=100 y=20 width=230 placeholder="Vor- und Nachname"
RnclButton    btnOk     text="OK" x=100 y=110 width=100
```

```bash
cargo run -p rncl --example designer -- --dump        # Eigenschaften aller Komponenten
cargo run -p rncl --example designer -- --screenshot form.png
```

## Vorschau-Bilder

`Widget::save_screenshot(pfad)` rendert jede Komponente samt Kindern in eine Bilddatei –
auch ohne Bildschirm (`xvfb-run`). Der Designer kann damit Vorschaubilder für Palette oder Dokumentation
erzeugen.
