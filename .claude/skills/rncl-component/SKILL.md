---
name: rncl-component
description: Neue RNCL-Komponente (UI-Element wie Button, Slider, Liste) für das RUF-Framework erstellen oder eine bestehende um Eigenschaften/Signale erweitern. Verwenden, wenn ein neues Widget, eine zusammengesetzte Komponente oder eine Drittanbieter-Komponente für RUF/RNCL/Freya TD gebaut werden soll.
---

# RNCL-Komponente erstellen

Eine RNCL-Komponente ist eine pure Rust-Struktur, die ein natives `ruf::Widget`
besitzt und `ruf::Component` implementiert. Referenzimplementierung:
`crates/rncl/src/button.rs` – vor dem Schreiben lesen und das Muster kopieren.

## Entscheidung: Welche Art Komponente?

1. **Zusammengesetzt** (aus vorhandenen Komponenten, z. B. Label + Textfeld):
   kein C++ nötig. Vorlage: `crates/rncl/examples/custom_component.rs` (`LabeledField`).
   Das äußere `RnclPanel` ist das `widget()` der Komponente.
2. **Steuerelement, das RUF schon erzeugen kann** (`WidgetKind`): nur Rust.
3. **Neues natives Steuerelement** (z. B. Datumswähler): zuerst alle drei Backends erweitern
   → Skill `ruf-backend`, danach hier weiter.

## Pflichtbestandteile (Checkliste)

- [ ] `#[derive(Debug, Clone)] pub struct RnclXxx { widget: Widget }`
      (`Clone` = nicht besitzender Verweis, für Closures).
- [ ] `pub const TYPE_NAME: &'static str = "RnclXxx";` – eindeutig in der Registry.
- [ ] Konstante je eigener Eigenschaft: `pub const VALUE: &'static str = "value";` (snake_case).
- [ ] `pub fn new<P: AsWidget + ?Sized>(parent: &P, …) -> Self` → ruft `with_parent`.
- [ ] `pub fn with_parent(parent: Option<&Widget>, …) -> Self` → `Widget::create(kind, parent)`;
      bei Text-Widgets danach `widget.adjust_size()`.
- [ ] Typisierte Methoden (`set_value`/`value`, `on_value_changed(impl FnMut(..) + 'static)`).
      Setter nehmen `&self` (Kern und System halten den Zustand).
- [ ] `pub fn info() -> ComponentInfo` mit `.category(..)`, `.description(..)`, ggf. `.container(true)`.
- [ ] `impl Component`: `widget`, `type_name`, und bei eigenen Eigenschaften
      `properties` (`base::descriptors()` + eigene), `property`, `set_property`
      – unbekannte Namen immer an `base::get` / `base::set` delegieren,
      Werte mit `base::expect_text/int/bool/text_list(name, &value)?` prüfen.
      Gleichartige Eigenschaften nicht neu schreiben, sondern die Helfer aus
      `crates/rncl/src/props.rs` nutzen (`text`, `items`, `range`, `orientation`);
      deren `set` liefert `None`, wenn der Name nicht passt → Kette mit
      `.or_else(..).unwrap_or_else(|| base::set(..))`, Vorbild `slider.rs`.
- [ ] In `crates/rncl/src/lib.rs`: `mod`, `pub use`, `register_all`, `prelude`,
      Tabelle im Crate-Doc.
- [ ] Test in `crates/rncl/tests/components.rs` ergänzen (Funktion schreiben **und**
      in die `tests`-Liste in `main` eintragen; `registry_create` prüft die Anzahl).
- [ ] Doku: `docs/de/komponenten.md` und `docs/en/components.md`.

## Gerüst

```rust
use ruf::{AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError,
          PropertyType, PropertyValue, Widget, WidgetKind, base};

#[derive(Debug, Clone)]
pub struct RnclXxx { widget: Widget }

impl RnclXxx {
    pub const TYPE_NAME: &'static str = "RnclXxx";
    pub const VALUE: &'static str = "value";

    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::Container, parent);
        Self { widget }
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |p| Box::new(Self::with_parent(p)))
            .category("Standard")
            .description("…")
    }
}

impl Component for RnclXxx {
    fn widget(&self) -> &Widget { &self.widget }
    fn type_name(&self) -> &'static str { Self::TYPE_NAME }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut p = base::descriptors();
        p.push(PropertyDescriptor::new(Self::VALUE, PropertyType::Int, "…"));
        p
    }
    fn property(&self, name: &str) -> Option<PropertyValue> {
        match name {
            Self::VALUE => Some(/* … */ 0.into()),
            _ => base::get(&self.widget, name),
        }
    }
    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            Self::VALUE => { let _v = base::expect_int(name, &value)?; /* … */ }
            _ => return base::set(&self.widget, name, value),
        }
        Ok(())
    }
}
```

## Regeln

- Kein `unsafe` in Komponenten; FFI gehört ausschließlich in `crates/ruf`.
- Keine Threads: Widgets sind `!Send`; alles läuft im GUI-Thread.
- Eigenschaftsnamen sind öffentliche API für den Designer – nicht umbenennen.
- Beschreibungstexte kurz und deutsch (sie erscheinen im Designer).

## Prüfen

```bash
cargo clippy --workspace --all-targets
cargo test --workspace
# Sichtprüfung ohne Bildschirm (Bild danach ansehen):
xvfb-run -a cargo run -p rncl --example designer -- --screenshot /tmp/form.png
```

Für eine Sichtprüfung der neuen Komponente: sie in einem Beispiel anlegen,
`window.show(); Application::process_events(); window.widget().save_screenshot(pfad)`.
