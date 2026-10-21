//! `RnclButton` – die Referenzimplementierung einer RNCL-Komponente.
//!
//! Wer eine eigene Komponente schreibt, kann diese Datei als Vorlage nehmen:
//! Struktur mit `Widget`, Konstruktor, eigene Methoden, `info()` für die
//! Registry und `impl Component` mit den zusätzlichen Eigenschaften.

use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyValue, Widget,
    WidgetKind, base,
};

use crate::text;

/// Eine Schaltfläche (`QPushButton`).
///
/// `clone()` liefert einen nicht besitzenden Verweis – ideal, um den Button
/// in einer Closure zu verändern.
#[derive(Debug, Clone)]
pub struct RnclButton {
    widget: Widget,
}

impl RnclButton {
    pub const TYPE_NAME: &'static str = "RnclButton";

    /// Erzeugt einen Button in `parent`; die Größe passt sich dem Text an.
    pub fn new<P: AsWidget + ?Sized>(parent: &P, text: &str) -> Self {
        Self::with_parent(Some(parent.as_widget()), text)
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>, text: &str) -> Self {
        let widget = Widget::create(WidgetKind::PushButton, parent);
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

    /// Wird bei jedem Klick aufgerufen. Mehrere Handler sind erlaubt.
    pub fn on_clicked(&self, f: impl FnMut() + 'static) {
        self.widget.on_clicked(f);
    }

    /// Löst einen Klick aus wie per Maus – praktisch für Tests und Tastenkürzel.
    pub fn click(&self) {
        self.widget.click();
    }

    /// Registry-Eintrag für den Designer.
    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent, Self::TYPE_NAME))
        })
        .category("Standard")
        .description("Schaltfläche, die beim Klick eine Aktion auslöst")
    }
}

impl Component for RnclButton {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.push(text::descriptor("Beschriftung"));
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
