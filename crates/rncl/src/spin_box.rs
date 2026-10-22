use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyType,
    PropertyValue, Widget, WidgetKind, base,
};

use crate::props::range;

/// Ein Zahlenfeld mit Pfeiltasten (`QSpinBox`). Standardbereich 0…99.
#[derive(Debug, Clone)]
pub struct RnclSpinBox {
    widget: Widget,
}

impl RnclSpinBox {
    pub const TYPE_NAME: &'static str = "RnclSpinBox";
    pub const READ_ONLY: &'static str = "read_only";

    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::SpinBox, parent);
        widget.adjust_size();
        let height = widget.size().height;
        widget.set_size(80, height);
        Self { widget }
    }

    /// Setzt den Wert (wird auf den Bereich begrenzt).
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

    pub fn set_read_only(&self, read_only: bool) {
        self.widget.set_read_only(read_only);
    }

    pub fn is_read_only(&self) -> bool {
        self.widget.is_read_only()
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
        .description("Ganzzahl-Eingabe mit Pfeiltasten")
    }
}

impl Component for RnclSpinBox {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.extend(range::descriptors());
        props.push(PropertyDescriptor::new(
            Self::READ_ONLY,
            PropertyType::Bool,
            "Nur lesbar",
        ));
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        match name {
            Self::READ_ONLY => Some(self.is_read_only().into()),
            _ => range::get(&self.widget, name).or_else(|| base::get(&self.widget, name)),
        }
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            Self::READ_ONLY => self.set_read_only(base::expect_bool(name, &value)?),
            _ => {
                return range::set(&self.widget, name, &value)
                    .unwrap_or_else(|| base::set(&self.widget, name, value));
            }
        }
        Ok(())
    }
}
