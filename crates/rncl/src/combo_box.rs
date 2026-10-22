use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyType,
    PropertyValue, Widget, WidgetKind, base,
};

use crate::props::{items, text};

/// Eine Auswahlliste zum Aufklappen (`QComboBox`).
#[derive(Debug, Clone)]
pub struct RnclComboBox {
    widget: Widget,
}

impl RnclComboBox {
    pub const TYPE_NAME: &'static str = "RnclComboBox";
    pub const EDITABLE: &'static str = "editable";

    /// Erzeugt eine leere Combobox (Standardbreite 160 px).
    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::ComboBox, parent);
        widget.adjust_size();
        let height = widget.size().height;
        widget.set_size(160, height);
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

    /// Wählt einen Eintrag aus; `-1` hebt die Auswahl auf.
    pub fn set_current_index(&self, index: i32) {
        self.widget.set_current_index(index);
    }

    pub fn current_index(&self) -> i32 {
        self.widget.current_index()
    }

    /// Text des ausgewählten Eintrags bzw. der freien Eingabe.
    pub fn current_text(&self) -> String {
        self.widget.text()
    }

    /// Wählt den Eintrag mit diesem Text aus (bzw. setzt den Text, wenn editierbar).
    pub fn set_current_text(&self, text: &str) {
        self.widget.set_text(text);
    }

    /// Erlaubt zusätzlich freie Texteingabe.
    pub fn set_editable(&self, editable: bool) {
        self.widget.set_editable(editable);
    }

    pub fn is_editable(&self) -> bool {
        self.widget.is_editable()
    }

    /// Grauer Hinweis, solange nichts ausgewählt ist.
    pub fn set_placeholder(&self, text: &str) {
        self.widget.set_placeholder(text);
    }

    /// Wird mit dem neuen Index aufgerufen (`-1` = keine Auswahl).
    pub fn on_current_index_changed(&self, f: impl FnMut(i32) + 'static) {
        self.widget.on_current_index_changed(f);
    }

    /// Wird mit dem neuen Text aufgerufen – bei Auswahl und bei freier Eingabe.
    pub fn on_text_changed(&self, f: impl FnMut(String) + 'static) {
        self.widget.on_text_changed(f);
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent))
        })
        .category("Eingabe")
        .description("Aufklappbare Auswahlliste")
    }
}

impl Component for RnclComboBox {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.extend(items::descriptors());
        props.push(text::descriptor("Aktueller Text"));
        props.push(PropertyDescriptor::new(
            Self::EDITABLE,
            PropertyType::Bool,
            "Freie Texteingabe erlauben",
        ));
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        match name {
            Self::EDITABLE => Some(self.is_editable().into()),
            _ => items::get(&self.widget, name)
                .or_else(|| text::get(&self.widget, name))
                .or_else(|| base::get(&self.widget, name)),
        }
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            text::TEXT => self.set_current_text(base::expect_text(name, &value)?),
            Self::EDITABLE => self.set_editable(base::expect_bool(name, &value)?),
            _ => {
                return items::set(&self.widget, name, &value)
                    .unwrap_or_else(|| base::set(&self.widget, name, value));
            }
        }
        Ok(())
    }
}
