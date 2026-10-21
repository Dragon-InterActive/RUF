use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyType,
    PropertyValue, Widget, WidgetKind, base,
};

use crate::text;

/// Ein Kontrollkästchen (`QCheckBox`).
#[derive(Debug, Clone)]
pub struct RnclCheckBox {
    widget: Widget,
}

impl RnclCheckBox {
    pub const TYPE_NAME: &'static str = "RnclCheckBox";
    pub const CHECKED: &'static str = "checked";

    pub fn new<P: AsWidget + ?Sized>(parent: &P, text: &str) -> Self {
        Self::with_parent(Some(parent.as_widget()), text)
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>, text: &str) -> Self {
        let widget = Widget::create(WidgetKind::CheckBox, parent);
        widget.set_text(text);
        widget.adjust_size();
        Self { widget }
    }

    pub fn set_text(&self, text: &str) {
        self.widget.set_text(text);
    }

    pub fn text(&self) -> String {
        self.widget.text()
    }

    pub fn set_checked(&self, checked: bool) {
        self.widget.set_checked(checked);
    }

    pub fn is_checked(&self) -> bool {
        self.widget.is_checked()
    }

    /// Wird bei jedem Wechsel des Häkchens aufgerufen – auch bei `set_checked`.
    pub fn on_toggled(&self, f: impl FnMut(bool) + 'static) {
        self.widget.on_toggled(f);
    }

    /// Schaltet um wie ein Mausklick.
    pub fn click(&self) {
        self.widget.click();
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent, Self::TYPE_NAME))
        })
        .category("Eingabe")
        .description("Kontrollkästchen für Ja/Nein-Werte")
    }
}

impl Component for RnclCheckBox {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.push(text::descriptor("Beschriftung"));
        props.push(PropertyDescriptor::new(
            Self::CHECKED,
            PropertyType::Bool,
            "Ist das Häkchen gesetzt?",
        ));
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        match name {
            Self::CHECKED => Some(self.is_checked().into()),
            _ => text::get(&self.widget, name).or_else(|| base::get(&self.widget, name)),
        }
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            text::TEXT => self.set_text(base::expect_text(name, &value)?),
            Self::CHECKED => self.set_checked(base::expect_bool(name, &value)?),
            _ => return base::set(&self.widget, name, value),
        }
        Ok(())
    }
}
