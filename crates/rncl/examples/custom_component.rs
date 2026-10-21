//! Eine eigene RNCL-Komponente schreiben und an das Framework übergeben.
//!
//! `LabeledField` setzt sich aus vorhandenen Komponenten zusammen (Panel +
//! Label + Textfeld), bringt eigene Eigenschaften mit und wird in der
//! `ComponentRegistry` angemeldet – genau wie die Standardkomponenten.
//!
//! cargo run -p rncl --example custom_component

use rncl::prelude::*;
use rncl::{RnclLabel, RnclPanel, RnclTextField};
use ruf::{PropertyDescriptor, PropertyError, PropertyType, base};

/// Beschriftung links, Eingabefeld rechts – als eine Komponente.
#[derive(Debug, Clone)]
pub struct LabeledField {
    panel: RnclPanel,
    label: RnclLabel,
    field: RnclTextField,
}

impl LabeledField {
    pub const TYPE_NAME: &'static str = "LabeledField";
    pub const CAPTION: &'static str = "caption";
    pub const VALUE: &'static str = "value";

    pub fn new<P: AsWidget + ?Sized>(parent: &P, caption: &str) -> Self {
        Self::with_parent(Some(parent.as_widget()), caption)
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>, caption: &str) -> Self {
        let panel = RnclPanel::with_parent(parent);
        panel.set_size(300, 28);
        let label = RnclLabel::new(&panel, caption);
        label.set_geometry(0, 4, 90, 20);
        let field = RnclTextField::new(&panel);
        field.set_geometry(95, 0, 205, 28);
        Self {
            panel,
            label,
            field,
        }
    }

    pub fn caption(&self) -> String {
        self.label.text()
    }

    pub fn set_caption(&self, caption: &str) {
        self.label.set_text(caption);
    }

    pub fn value(&self) -> String {
        self.field.text()
    }

    pub fn set_value(&self, value: &str) {
        self.field.set_text(value);
    }

    pub fn on_value_changed(&self, f: impl FnMut(String) + 'static) {
        self.field.on_text_changed(f);
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent, "Feld:"))
        })
        .category("Eigene")
        .description("Beschriftetes Eingabefeld")
    }
}

impl Component for LabeledField {
    // Das äußere Panel ist "das" Widget der Komponente: Position, Größe usw.
    // wirken auf die ganze Gruppe.
    fn widget(&self) -> &Widget {
        self.panel.widget()
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.push(PropertyDescriptor::new(
            Self::CAPTION,
            PropertyType::Text,
            "Beschriftung",
        ));
        props.push(PropertyDescriptor::new(
            Self::VALUE,
            PropertyType::Text,
            "Eingabewert",
        ));
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        match name {
            Self::CAPTION => Some(self.caption().into()),
            Self::VALUE => Some(self.value().into()),
            _ => base::get(self.widget(), name),
        }
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            Self::CAPTION => self.set_caption(base::expect_text(name, &value)?),
            Self::VALUE => self.set_value(base::expect_text(name, &value)?),
            _ => return base::set(self.widget(), name, value),
        }
        Ok(())
    }
}

fn main() {
    let app = Application::new();

    // Eigene Komponente neben den Standardkomponenten anmelden.
    let mut registry = rncl::registry();
    registry
        .register(LabeledField::info())
        .expect("Typname ist eindeutig");
    for info in registry.iter() {
        println!(
            "{:<14} [{}] {}",
            info.type_name, info.category, info.description
        );
    }

    let window = RnclWindow::new("Eigene Komponente");
    window.set_size(340, 160);

    // Direkt benutzen …
    let name = LabeledField::new(&window, "Name:");
    name.set_position(20, 20);

    // … oder – wie der Designer – über die Registry per Typname.
    let mut city = registry
        .create(LabeledField::TYPE_NAME, Some(window.widget()))
        .unwrap();
    city.set_property("caption", "Ort:".into()).unwrap();
    city.set_property("y", 60.into()).unwrap();
    city.set_property("x", 20.into()).unwrap();

    let echo = RnclLabel::new(&window, "");
    echo.set_geometry(20, 110, 300, 24);
    let e = echo.clone();
    name.on_value_changed(move |v| e.set_text(&format!("Hallo {v}!")));

    window.show();
    std::process::exit(app.exec());
}
