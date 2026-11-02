use ruf::{
    AsWidget, Component, ComponentInfo, KeyEvent, MouseEvent, PropertyDescriptor, PropertyError,
    PropertyValue, Shape, Widget, WidgetKind, base,
};

/// Eine Zeichenfläche: malt [`Shape`]s und meldet Maus und Tastatur –
/// die Grundlage eigener Ansichten wie der Designfläche von Freya TD.
#[derive(Debug, Clone)]
pub struct RnclCanvas {
    widget: Widget,
}

impl RnclCanvas {
    pub const TYPE_NAME: &'static str = "RnclCanvas";

    /// Erzeugt eine leere Zeichenfläche (Standardgröße 200×150).
    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::Canvas, parent);
        widget.set_size(200, 150);
        Self { widget }
    }

    /// Ersetzt, was gemalt wird, und zeichnet neu.
    pub fn set_shapes(&self, shapes: Vec<Shape>) {
        self.widget.set_shapes(shapes);
    }

    pub fn shapes(&self) -> Vec<Shape> {
        self.widget.shapes()
    }

    /// Drücken, Loslassen, Bewegen, Doppelklick; ein Klick gibt den Fokus.
    pub fn on_mouse(&self, f: impl FnMut(MouseEvent) + 'static) {
        self.widget.on_mouse(f);
    }

    /// Tastendrücke, solange die Zeichenfläche den Fokus hat.
    pub fn on_key(&self, f: impl FnMut(KeyEvent) + 'static) {
        self.widget.on_key(f);
    }

    /// Getippter Text über die Eingabemethode (tote Tasten, Akzente, IME);
    /// siehe [`ruf::Widget::on_text_input`].
    pub fn on_text_input(&self, f: impl FnMut(String) + 'static) {
        self.widget.on_text_input(f);
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent))
        })
        .category("Anzeige")
        .description("Zeichenfläche für eigene Darstellungen mit Maus und Tastatur")
    }
}

impl Component for RnclCanvas {
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
