use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyValue, TreeItem,
    Widget, WidgetKind, base,
};

/// Eine Baumansicht mit auf- und zuklappbaren Knoten (z. B. Projekt- oder
/// Komponentenstruktur). Knoten werden über ihre Kennung angesprochen.
#[derive(Debug, Clone)]
pub struct RnclTreeView {
    widget: Widget,
}

impl RnclTreeView {
    pub const TYPE_NAME: &'static str = "RnclTreeView";

    /// Erzeugt eine leere Baumansicht (Standardgröße 200×240).
    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::TreeView, parent);
        widget.set_size(200, 240);
        Self { widget }
    }

    /// Ersetzt alle Knoten; die Auswahl bleibt, wenn ihr Knoten sichtbar bleibt.
    pub fn set_items(&self, items: Vec<TreeItem>) {
        self.widget.set_tree(items);
    }

    pub fn items(&self) -> Vec<TreeItem> {
        self.widget.tree()
    }

    /// Kennungen der sichtbaren Zeilen.
    pub fn visible_items(&self) -> Vec<String> {
        self.widget.visible_tree_items()
    }

    pub fn toggle(&self, id: &str) -> bool {
        self.widget.toggle_tree_item(id)
    }

    pub fn select(&self, id: Option<&str>) {
        self.widget.select_tree_item(id);
    }

    pub fn selected(&self) -> Option<String> {
        self.widget.selected_tree_item()
    }

    /// Auswahlwechsel mit der Kennung (leer = keine Auswahl).
    pub fn on_selection_changed(&self, f: impl FnMut(String) + 'static) {
        self.widget.on_tree_selection_changed(f);
    }

    /// Ein Knoten wurde auf- (`true`) oder zugeklappt.
    pub fn on_item_toggled(&self, f: impl FnMut(String, bool) + 'static) {
        self.widget.on_tree_item_toggled(f);
    }

    /// Doppelklick bzw. Enter auf einem Knoten.
    pub fn on_item_activated(&self, f: impl FnMut(String) + 'static) {
        self.widget.on_tree_item_activated(f);
    }

    /// Rechtsklick auf einen Knoten (er ist dann gewählt): Kennung und
    /// Position relativ zur Baumansicht – passend für
    /// `widget().popup_menu(&menu, x, y)`; die Wahl meldet `on_menu` des Fensters.
    pub fn on_context_menu(&self, mut f: impl FnMut(String, i32, i32) + 'static) {
        let widget = self.widget.clone();
        self.widget.on_mouse(move |e| {
            if e.button == 3
                && let Some(id) = widget.selected_tree_item()
            {
                f(id, e.x, e.y);
            }
        });
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent))
        })
        .category("Eingabe")
        .description("Baumansicht mit auf- und zuklappbaren Knoten")
    }
}

impl Component for RnclTreeView {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        base::descriptors()
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        base::get(&self.widget, name)
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        base::set(&self.widget, name, value)
    }
}
