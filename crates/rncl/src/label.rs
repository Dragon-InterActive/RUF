use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyValue, Widget,
    WidgetKind, base,
};

use crate::text;

/// Ein statischer Text (`QLabel`).
#[derive(Debug, Clone)]
pub struct RnclLabel {
    widget: Widget,
}

impl RnclLabel {
    pub const TYPE_NAME: &'static str = "RnclLabel";

    pub fn new<P: AsWidget + ?Sized>(parent: &P, text: &str) -> Self {
        Self::with_parent(Some(parent.as_widget()), text)
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>, text: &str) -> Self {
        let widget = Widget::create(WidgetKind::Label, parent);
        widget.set_text(text);
        widget.adjust_size();
        Self { widget }
    }

    /// Setzt den Text. Qt erkennt einfaches HTML (z. B. `<b>fett</b>`) automatisch.
    pub fn set_text(&self, text: &str) {
        self.widget.set_text(text);
    }

    pub fn text(&self) -> String {
        self.widget.text()
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent, Self::TYPE_NAME))
        })
        .category("Standard")
        .description("Statischer Text")
    }
}

impl Component for RnclLabel {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.push(text::descriptor("Angezeigter Text"));
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        text::get(&self.widget, name).or_else(|| base::get(&self.widget, name))
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            text::TEXT => self.set_text(base::expect_text(name, &value)?),
            _ => return base::set(&self.widget, name, value),
        }
        Ok(())
    }
}
