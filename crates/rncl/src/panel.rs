use ruf::{AsWidget, Component, ComponentInfo, Widget, WidgetKind};

/// Ein schlichter Container ohne eigenes Aussehen.
///
/// Gruppiert Komponenten (Positionen sind relativ zum Panel) und ist die
/// Basis für zusammengesetzte eigene Komponenten.
#[derive(Debug, Clone)]
pub struct RnclPanel {
    widget: Widget,
}

impl RnclPanel {
    pub const TYPE_NAME: &'static str = "RnclPanel";

    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::Container, parent);
        widget.set_size(200, 100);
        Self { widget }
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent))
        })
        .category("Container")
        .description("Unsichtbarer Container zum Gruppieren von Komponenten")
        .container(true)
    }
}

impl Component for RnclPanel {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }
}
