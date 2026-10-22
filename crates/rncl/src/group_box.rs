use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyType,
    PropertyValue, Widget, WidgetKind, base,
};

/// Ein Container mit Rahmen und Titel (`QGroupBox`).
///
/// Gruppiert zusammengehörige Komponenten sichtbar und macht enthaltene
/// [`RnclRadioButton`](crate::RnclRadioButton)s zu einer eigenen Auswahlgruppe.
/// Kinder-Positionen sind relativ zur Box – der Titel belegt oben etwa 20 px.
#[derive(Debug, Clone)]
pub struct RnclGroupBox {
    widget: Widget,
}

impl RnclGroupBox {
    pub const TYPE_NAME: &'static str = "RnclGroupBox";
    pub const TITLE: &'static str = "title";

    pub fn new<P: AsWidget + ?Sized>(parent: &P, title: &str) -> Self {
        Self::with_parent(Some(parent.as_widget()), title)
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>, title: &str) -> Self {
        let widget = Widget::create(WidgetKind::GroupBox, parent);
        widget.set_text(title);
        widget.set_size(200, 100);
        Self { widget }
    }

    pub fn set_title(&self, title: &str) {
        self.widget.set_text(title);
    }

    pub fn title(&self) -> String {
        self.widget.text()
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent, "Gruppe"))
        })
        .category("Container")
        .description("Container mit Rahmen und Titel")
        .container(true)
    }
}

impl Component for RnclGroupBox {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.push(PropertyDescriptor::new(
            Self::TITLE,
            PropertyType::Text,
            "Titel im Rahmen",
        ));
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        match name {
            Self::TITLE => Some(self.title().into()),
            _ => base::get(&self.widget, name),
        }
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            Self::TITLE => self.set_title(base::expect_text(name, &value)?),
            _ => return base::set(&self.widget, name, value),
        }
        Ok(())
    }
}
