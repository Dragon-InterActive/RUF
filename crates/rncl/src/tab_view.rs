use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyType,
    PropertyValue, Widget, WidgetKind, base,
};

use crate::RnclPanel;

/// Registerkarten (`QTabWidget`): mehrere Seiten, von denen eine sichtbar ist.
///
/// Jede Seite ist ein [`RnclPanel`]; Größe und Position der Seiten verwaltet
/// die Registerkarte selbst.
///
/// ```no_run
/// # use rncl::prelude::*;
/// # let app = Application::new();
/// # let window = RnclWindow::new("t");
/// let tabs = RnclTabView::new(&window);
/// let general = tabs.add_tab("Allgemein");
/// RnclCheckBox::new(&general, "Autostart").set_position(10, 10);
/// ```
#[derive(Debug, Clone)]
pub struct RnclTabView {
    widget: Widget,
}

impl RnclTabView {
    pub const TYPE_NAME: &'static str = "RnclTabView";
    pub const TABS: &'static str = "tabs";
    pub const CURRENT_INDEX: &'static str = "current_index";

    /// Erzeugt eine Registerkarte ohne Seiten (Standardgröße 300×200).
    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::TabView, parent);
        widget.set_size(300, 200);
        Self { widget }
    }

    /// Fügt eine Seite hinzu und liefert sie als Container für weitere Komponenten.
    pub fn add_tab(&self, title: &str) -> RnclPanel {
        RnclPanel::from_widget(self.widget.add_tab(title))
    }

    /// Entfernt eine Seite samt Inhalt.
    pub fn remove_tab(&self, index: i32) {
        self.widget.remove_item(index);
    }

    pub fn tab_count(&self) -> i32 {
        self.widget.item_count()
    }

    pub fn tab_title(&self, index: i32) -> String {
        self.widget.item_text(index)
    }

    pub fn set_tab_title(&self, index: i32, title: &str) {
        self.widget.set_item_text(index, title);
    }

    /// Alle Seitentitel.
    pub fn tab_titles(&self) -> Vec<String> {
        self.widget.items()
    }

    /// Passt die Seiten an `titles` an: vorhandene werden umbenannt, fehlende
    /// angelegt, überzählige (samt Inhalt) entfernt. So bleibt der Inhalt
    /// bestehender Seiten erhalten.
    pub fn set_tab_titles<S: AsRef<str>>(&self, titles: &[S]) {
        for (i, title) in titles.iter().enumerate() {
            let i = i as i32;
            if i < self.tab_count() {
                self.set_tab_title(i, title.as_ref());
            } else {
                self.add_tab(title.as_ref());
            }
        }
        while self.tab_count() > titles.len() as i32 {
            self.remove_tab(self.tab_count() - 1);
        }
    }

    /// Zeigt die Seite mit diesem Index.
    pub fn set_current_index(&self, index: i32) {
        self.widget.set_current_index(index);
    }

    pub fn current_index(&self) -> i32 {
        self.widget.current_index()
    }

    /// Wird beim Seitenwechsel mit dem neuen Index aufgerufen.
    pub fn on_current_index_changed(&self, f: impl FnMut(i32) + 'static) {
        self.widget.on_current_index_changed(f);
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            let tabs = Self::with_parent(parent);
            tabs.add_tab("Seite 1");
            Box::new(tabs)
        })
        .category("Container")
        .description("Registerkarten mit mehreren Seiten")
        .container(true)
    }
}

impl Component for RnclTabView {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.push(PropertyDescriptor::new(
            Self::TABS,
            PropertyType::TextList,
            "Seitentitel",
        ));
        props.push(PropertyDescriptor::new(
            Self::CURRENT_INDEX,
            PropertyType::Int,
            "Sichtbare Seite",
        ));
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        match name {
            Self::TABS => Some(self.tab_titles().into()),
            Self::CURRENT_INDEX => Some(self.current_index().into()),
            _ => base::get(&self.widget, name),
        }
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            Self::TABS => self.set_tab_titles(base::expect_text_list(name, &value)?),
            Self::CURRENT_INDEX => self.set_current_index(base::expect_int(name, &value)?),
            _ => return base::set(&self.widget, name, value),
        }
        Ok(())
    }
}
