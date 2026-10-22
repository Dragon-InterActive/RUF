use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyValue, Widget,
    WidgetKind, base,
};

use crate::props::{orientation, range};

/// Ein Schieberegler (`QSlider`), standardmäßig waagerecht. Standardbereich 0…99.
#[derive(Debug, Clone)]
pub struct RnclSlider {
    widget: Widget,
}

impl RnclSlider {
    pub const TYPE_NAME: &'static str = "RnclSlider";

    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::Slider, parent);
        widget.adjust_size();
        let height = widget.size().height;
        widget.set_size(160, height);
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

    /// Senkrecht statt waagerecht. Breite und Höhe ggf. selbst tauschen.
    pub fn set_vertical(&self, vertical: bool) {
        self.widget.set_vertical(vertical);
    }

    pub fn is_vertical(&self) -> bool {
        self.widget.is_vertical()
    }

    /// Wird bei jeder Wertänderung aufgerufen – auch bei `set_value`.
    pub fn on_value_changed(&self, f: impl FnMut(i32) + 'static) {
        self.widget.on_value_changed(f);
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent))
        })
        .category("Eingabe")
        .description("Schieberegler für Ganzzahlen")
    }
}

impl Component for RnclSlider {
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
