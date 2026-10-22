use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyValue, Widget,
    WidgetKind, base,
};

use crate::props::items;

/// Eine Liste mit auswählbaren Einträgen (`QListWidget`).
#[derive(Debug, Clone)]
pub struct RnclListBox {
    widget: Widget,
}

impl RnclListBox {
    pub const TYPE_NAME: &'static str = "RnclListBox";

    /// Erzeugt eine leere Liste (Standardgröße 160×120).
    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::ListBox, parent);
        widget.set_size(160, 120);
        Self { widget }
    }

    pub fn add_item(&self, text: &str) {
        self.widget.add_item(text);
    }

    pub fn insert_item(&self, index: i32, text: &str) {
        self.widget.insert_item(index, text);
    }

    pub fn remove_item(&self, index: i32) {
        self.widget.remove_item(index);
    }

    pub fn clear(&self) {
        self.widget.clear_items();
    }

    /// Ersetzt alle Einträge.
    pub fn set_items<S: AsRef<str>>(&self, items: &[S]) {
        self.widget.set_items(items);
    }

    pub fn items(&self) -> Vec<String> {
        self.widget.items()
    }

    pub fn count(&self) -> i32 {
        self.widget.item_count()
    }

    pub fn item_text(&self, index: i32) -> String {
        self.widget.item_text(index)
    }

    pub fn set_item_text(&self, index: i32, text: &str) {
        self.widget.set_item_text(index, text);
    }

    /// Wählt eine Zeile aus; `-1` hebt die Auswahl auf.
    pub fn set_current_index(&self, index: i32) {
        self.widget.set_current_index(index);
    }

    pub fn current_index(&self) -> i32 {
        self.widget.current_index()
    }

    /// Text der ausgewählten Zeile (`None` ohne Auswahl).
    pub fn current_text(&self) -> Option<String> {
        let index = self.current_index();
        (index >= 0).then(|| self.item_text(index))
    }

    /// Wird mit der neu ausgewählten Zeile aufgerufen (`-1` = keine Auswahl).
    pub fn on_current_index_changed(&self, f: impl FnMut(i32) + 'static) {
        self.widget.on_current_index_changed(f);
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent))
        })
        .category("Eingabe")
        .description("Liste mit auswählbaren Einträgen")
    }
}

impl Component for RnclListBox {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.extend(items::descriptors());
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        items::get(&self.widget, name).or_else(|| base::get(&self.widget, name))
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        items::set(&self.widget, name, &value)
            .unwrap_or_else(|| base::set(&self.widget, name, value))
    }
}
