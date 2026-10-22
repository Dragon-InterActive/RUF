use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyType,
    PropertyValue, Widget, WidgetKind, base,
};

use crate::props::text;

/// Ein Optionsfeld (`QRadioButton`).
///
/// Optionsfelder mit demselben Elternteil schließen sich gegenseitig aus. Für
/// mehrere unabhängige Gruppen je Gruppe eine [`RnclGroupBox`](crate::RnclGroupBox)
/// oder ein [`RnclPanel`](crate::RnclPanel) als Elternteil verwenden.
#[derive(Debug, Clone)]
pub struct RnclRadioButton {
    widget: Widget,
}

impl RnclRadioButton {
    pub const TYPE_NAME: &'static str = "RnclRadioButton";
    pub const CHECKED: &'static str = "checked";

    pub fn new<P: AsWidget + ?Sized>(parent: &P, text: &str) -> Self {
        Self::with_parent(Some(parent.as_widget()), text)
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>, text: &str) -> Self {
        let widget = Widget::create(WidgetKind::RadioButton, parent);
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

    /// Wählt diese Option aus (die übrigen der Gruppe werden abgewählt).
    pub fn set_checked(&self, checked: bool) {
        self.widget.set_checked(checked);
    }

    pub fn is_checked(&self) -> bool {
        self.widget.is_checked()
    }

    /// Wird beim Aus- **und** Abwählen aufgerufen.
    pub fn on_toggled(&self, f: impl FnMut(bool) + 'static) {
        self.widget.on_toggled(f);
    }

    /// Wählt aus wie ein Mausklick.
    pub fn click(&self) {
        self.widget.click();
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent, Self::TYPE_NAME))
        })
        .category("Eingabe")
        .description("Optionsfeld – genau eine Option je Gruppe")
    }
}

impl Component for RnclRadioButton {
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
            "Ist die Option ausgewählt?",
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
