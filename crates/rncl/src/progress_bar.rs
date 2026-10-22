use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyValue, Widget,
    WidgetKind, base,
};

use crate::props::{orientation, range};

/// Ein Fortschrittsbalken (`QProgressBar`). Standardbereich 0…100, Startwert 0.
///
/// Sind `minimum` und `maximum` beide 0, zeigt das System einen "beschäftigt"-Balken
/// ohne konkreten Fortschritt an.
#[derive(Debug, Clone)]
pub struct RnclProgressBar {
    widget: Widget,
}

impl RnclProgressBar {
    pub const TYPE_NAME: &'static str = "RnclProgressBar";

    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::ProgressBar, parent);
        widget.set_size(160, 24);
        Self { widget }
    }

    pub fn set_value(&self, value: i32) {
        self.widget.set_value(value);
    }

    pub fn value(&self) -> i32 {
        self.widget.value()
    }

    pub fn set_range(&self, minimum: i32, maximum: i32) {
        self.widget.set_range(minimum, maximum);
    }

    pub fn minimum(&self) -> i32 {
        self.widget.minimum()
    }

    pub fn maximum(&self) -> i32 {
        self.widget.maximum()
    }

    /// Schaltet in den "beschäftigt"-Modus (`true`) oder zurück auf 0…100.
    pub fn set_busy(&self, busy: bool) {
        if busy {
            self.set_range(0, 0);
        } else {
            self.set_range(0, 100);
        }
    }

    pub fn set_vertical(&self, vertical: bool) {
        self.widget.set_vertical(vertical);
    }

    pub fn is_vertical(&self) -> bool {
        self.widget.is_vertical()
    }

    pub fn on_value_changed(&self, f: impl FnMut(i32) + 'static) {
        self.widget.on_value_changed(f);
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent))
        })
        .category("Anzeige")
        .description("Fortschrittsbalken")
    }
}

impl Component for RnclProgressBar {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.extend(range::descriptors());
        props.push(orientation::descriptor());
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        range::get(&self.widget, name)
            .or_else(|| orientation::get(&self.widget, name))
            .or_else(|| base::get(&self.widget, name))
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        range::set(&self.widget, name, &value)
            .or_else(|| orientation::set(&self.widget, name, &value))
            .unwrap_or_else(|| base::set(&self.widget, name, value))
    }
}
