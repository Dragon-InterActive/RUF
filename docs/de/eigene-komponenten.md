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

## Variante B: neues natives Steuerelement

Braucht man ein Steuerelement, das RUF noch nicht kennt (z. B. einen Datumswähler),
wird RUF selbst erweitert – im GTK-Backend, das auf allen Systemen läuft:

1. `WidgetKind` in `crates/ruf/src/core.rs` um eine Variante mit der **nächsten freien
   Zahl** ergänzen (Werte nie umnummerieren).
2. Im Backend (`backend/gtk.rs`) den Fall
   in `create` und die betroffenen Funktionen des `Backend`-Traits umsetzen.
3. Braucht das Steuerelement neue Fähigkeiten, den Trait `Backend` erweitern.
4. Danach die RNCL-Komponente wie in Variante A, aber mit `Widget::create(WidgetKind::…)`.

Schritt für Schritt im Skill `.claude/skills/ruf-backend/SKILL.md`.
