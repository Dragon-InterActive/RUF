# Custom components

RNCL is open: any crate can provide components that behave exactly like the built-in
ones – in code and in the designer. All it takes is implementing `ruf::Component` and
offering a `ComponentInfo` entry.

## Option A: composite component (no C++)

Complete, runnable example: `crates/rncl/examples/custom_component.rs`.

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
        ComponentInfo::new(Self::TYPE_NAME, |p| Box::new(Self::with_parent(p, "Field:")))
            .category("Custom")
            .description("Labelled input field")
    }
}

impl Component for LabeledField {
    fn widget(&self) -> &Widget { self.panel.widget() }   // the outer panel
    fn type_name(&self) -> &'static str { Self::TYPE_NAME }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut p = base::descriptors();
        p.push(PropertyDescriptor::new(Self::CAPTION, PropertyType::Text, "Caption"));
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

## Registering the component

```rust
let mut registry = rncl::registry();          // all standard components
registry.register(LabeledField::info())?;     // plus your own
let c = registry.create("LabeledField", Some(window.widget()))?;
```

A library with several components should – like `rncl` – offer a
`register_all(&mut ComponentRegistry)` function.

## Checklist

- Choose a globally unique `TYPE_NAME` (prefix with your library, e.g. `"AcmeGauge"`).
- Derive `Clone` (non-owning references for closures).
- Provide both `new(&parent, …)` **and** `with_parent(Option<&Widget>, …)`.
- Delegate unknown property names to `base::get` / `base::set`.
- Validate values with `base::expect_text` / `expect_int` / `expect_bool` – this yields
  uniform errors (`PropertyError::TypeMismatch`).
- Property names are public API (designer files store them) – keep them stable.

## Option B: a new native control

If you need a control RUF does not know yet (e.g. a date picker), extend RUF itself –
in the GTK backend, which runs on all systems:

1. Add a variant with the **next free number** to `WidgetKind` in
   `crates/ruf/src/core.rs` (never renumber values).
2. In the backend (`backend/gtk.rs`) implement
   the case in `create` and the affected functions of the `Backend` trait.
3. If the control needs new capabilities, extend the `Backend` trait.
4. Then build the RNCL component as in option A, but with `Widget::create(WidgetKind::…)`.

Step by step in the skill `.claude/skills/ruf-backend/SKILL.md`.
