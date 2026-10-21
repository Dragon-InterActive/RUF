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

## Option B: a new native Qt widget

If you need a Qt widget RUF does not know yet (e.g. `QSlider`):

1. **Contribute to RUF** (recommended): extend `WidgetKind` and the bridge – step by
   step in the skill `.claude/skills/ruf-bridge/SKILL.md`.
2. **Your own bridge in your own crate**: RUF exports its headers and the `QWidget`
   type. Your `#[cxx_qt::bridge]` creates the widget in C++ and returns a
   `*mut QWidget`; `unsafe { Widget::from_raw(ptr) }` turns it into a `ruf::Widget`:

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

   // SAFETY: pointer comes from a live ruf::Widget or fresh from C++.
   let raw = unsafe { ffi::acme_gauge_new(parent.as_widget().as_ptr()) };
   let widget = unsafe { ruf::Widget::from_raw(raw) };   // the parent owns the widget
   ```

   Your `build.rs` uses `cxx_qt_build::CxxQtBuilder` and automatically inherits Qt
   modules and include paths from `ruf`.
