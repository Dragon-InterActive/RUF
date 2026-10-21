# Eigene Komponenten

RNCL ist offen: Jede Crate kann Komponenten bereitstellen, die sich genauso verhalten
wie die eingebauten – im Code wie im Designer. Dazu genügt es, `ruf::Component` zu
implementieren und einen `ComponentInfo`-Eintrag anzubieten.

## Variante A: zusammengesetzte Komponente (kein C++)

Vollständiges, lauffähiges Beispiel: `crates/rncl/examples/custom_component.rs`.

```rust
use rncl::prelude::*;
use rncl::{RnclLabel, RnclPanel, RnclTextField};
use ruf::{base, PropertyDescriptor, PropertyError, PropertyType};

#[derive(Debug, Clone)]
pub struct LabeledField {
    panel: RnclPanel,
    label: RnclLabel,
    field: RnclTextField,
}

impl LabeledField {
    pub const TYPE_NAME: &'static str = "LabeledField";
    pub const CAPTION: &'static str = "caption";

    pub fn new<P: AsWidget + ?Sized>(parent: &P, caption: &str) -> Self {
        Self::with_parent(Some(parent.as_widget()), caption)
    }

    pub fn with_parent(parent: Option<&Widget>, caption: &str) -> Self {
        let panel = RnclPanel::with_parent(parent);
        panel.set_size(300, 28);
        let label = RnclLabel::new(&panel, caption);
        label.set_geometry(0, 4, 90, 20);
        let field = RnclTextField::new(&panel);
        field.set_geometry(95, 0, 205, 28);
        Self { panel, label, field }
    }

    pub fn value(&self) -> String { self.field.text() }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |p| Box::new(Self::with_parent(p, "Feld:")))
            .category("Eigene")
            .description("Beschriftetes Eingabefeld")
    }
}

impl Component for LabeledField {
    fn widget(&self) -> &Widget { self.panel.widget() }   // das äußere Panel
    fn type_name(&self) -> &'static str { Self::TYPE_NAME }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut p = base::descriptors();
        p.push(PropertyDescriptor::new(Self::CAPTION, PropertyType::Text, "Beschriftung"));
        p
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        match name {
            Self::CAPTION => Some(self.label.text().into()),
            _ => base::get(self.widget(), name),
        }
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            Self::CAPTION => self.label.set_text(base::expect_text(name, &value)?),
            _ => return base::set(self.widget(), name, value),
        }
        Ok(())
    }
}
```

## Komponente anmelden

```rust
let mut registry = rncl::registry();          // alle Standardkomponenten
registry.register(LabeledField::info())?;     // eigene dazu
let c = registry.create("LabeledField", Some(window.widget()))?;
```

Eine Bibliothek mit mehreren Komponenten bietet am besten – wie `rncl` – eine Funktion
`register_all(&mut ComponentRegistry)` an.

## Checkliste

- `TYPE_NAME` global eindeutig wählen (Präfix der eigenen Bibliothek, z. B. `"AcmeGauge"`).
- `Clone` ableiten (nicht besitzende Verweise für Closures).
- `new(&parent, …)` **und** `with_parent(Option<&Widget>, …)` anbieten.
- Unbekannte Eigenschaftsnamen an `base::get` / `base::set` delegieren.
- Werte mit `base::expect_text` / `expect_int` / `expect_bool` prüfen – so entstehen
  einheitliche Fehlermeldungen (`PropertyError::TypeMismatch`).
- Eigenschaftsnamen sind öffentliche API (Designer-Dateien speichern sie) – stabil halten.

## Variante B: neues natives Qt-Widget

Benötigt man ein Qt-Widget, das RUF noch nicht kennt (z. B. `QSlider`):

1. **In RUF beitragen** (empfohlen): `WidgetKind` erweitern und die Brücke ergänzen –
   Schritt für Schritt im Skill `.claude/skills/ruf-bridge/SKILL.md`.
2. **Eigene Bridge in der eigenen Crate**: RUF exportiert seine Header und den Typ
   `QWidget`. Die eigene `#[cxx_qt::bridge]` erzeugt das Widget in C++ und gibt einen
   `*mut QWidget` zurück; daraus wird mit `unsafe { Widget::from_raw(ptr) }` ein
   `ruf::Widget`:

   ```rust
   #[cxx_qt::bridge]
   mod ffi {
       unsafe extern "C++" {
           include!("ruf/cpp/ruf_widgets.h");
           type QWidget = ruf::sys::QWidget;
           include!("acme/gauge.h");
           unsafe fn acme_gauge_new(parent: *mut QWidget) -> *mut QWidget;
       }
   }

   // SAFETY: Zeiger stammt aus einem lebenden ruf::Widget bzw. frisch aus C++.
   let raw = unsafe { ffi::acme_gauge_new(parent.as_widget().as_ptr()) };
   let widget = unsafe { ruf::Widget::from_raw(raw) };   // Elternteil besitzt das Widget
   ```

   Die eigene `build.rs` nutzt `cxx_qt_build::CxxQtBuilder` und erbt Qt-Module und
   Include-Pfade automatisch von `ruf`.
