use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyType,
    PropertyValue, Widget, WidgetKind, base,
};

use crate::text;

/// Ein einzeiliges Eingabefeld (`QLineEdit`).
#[derive(Debug, Clone)]
pub struct RnclTextField {
    widget: Widget,
}

impl RnclTextField {
    pub const TYPE_NAME: &'static str = "RnclTextField";
    pub const PLACEHOLDER: &'static str = "placeholder";
    pub const PASSWORD: &'static str = "password";

    /// Erzeugt ein leeres Eingabefeld (Standardbreite 160 px).
    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::LineEdit, parent);
        widget.adjust_size();
        let height = widget.size().height;
        widget.set_size(160, height);
        Self { widget }
    }

    pub fn set_text(&self, text: &str) {
        self.widget.set_text(text);
    }

    pub fn text(&self) -> String {
        self.widget.text()
    }

    /// Grauer Hinweistext, solange das Feld leer ist.
    pub fn set_placeholder(&self, text: &str) {
        self.widget.set_placeholder(text);
    }

    pub fn placeholder(&self) -> String {
        self.widget.placeholder()
    }

    /// Verdeckt die Eingabe (für Passwörter).
    pub fn set_password_mode(&self, enabled: bool) {
        self.widget.set_password_mode(enabled);
    }

    pub fn is_password_mode(&self) -> bool {
        self.widget.is_password_mode()
    }

    /// Wird bei jeder Änderung mit dem neuen Text aufgerufen – auch bei `set_text`.
    pub fn on_text_changed(&self, f: impl FnMut(String) + 'static) {
        self.widget.on_text_changed(f);
    }

    /// Wird bei der Enter-Taste aufgerufen.
    pub fn on_return_pressed(&self, f: impl FnMut() + 'static) {
        self.widget.on_return_pressed(f);
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent))
        })
        .category("Eingabe")
        .description("Einzeiliges Texteingabefeld")
    }
}

impl Component for RnclTextField {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.push(text::descriptor("Eingegebener Text"));
        props.push(PropertyDescriptor::new(
            Self::PLACEHOLDER,
            PropertyType::Text,
            "Hinweistext bei leerem Feld",
        ));
        props.push(PropertyDescriptor::new(
            Self::PASSWORD,
            PropertyType::Bool,
            "Eingabe verdecken (Passwortfeld)",
        ));
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        match name {
            Self::PLACEHOLDER => Some(self.placeholder().into()),
            Self::PASSWORD => Some(self.is_password_mode().into()),
            _ => text::get(&self.widget, name).or_else(|| base::get(&self.widget, name)),
        }
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            text::TEXT => self.set_text(base::expect_text(name, &value)?),
            Self::PLACEHOLDER => self.set_placeholder(base::expect_text(name, &value)?),
            Self::PASSWORD => self.set_password_mode(base::expect_bool(name, &value)?),
            _ => return base::set(&self.widget, name, value),
        }
        Ok(())
    }
}
