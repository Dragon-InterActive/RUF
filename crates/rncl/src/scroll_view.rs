use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyType,
    PropertyValue, Widget, WidgetKind, base,
};

use crate::RnclPanel;

/// Eine Scrollfläche (`QScrollArea`) mit einem [`RnclPanel`] als Inhalt.
///
/// Der Inhalt füllt den sichtbaren Bereich aus, wird aber nie kleiner als
/// [`RnclScrollView::set_content_size`]. Ist er größer als der sichtbare
/// Bereich, erscheinen Bildlaufleisten.
///
/// ```no_run
/// # use rncl::prelude::*;
/// # let app = Application::new();
/// # let window = RnclWindow::new("t");
/// let scroll = RnclScrollView::new(&window);
/// scroll.set_geometry(10, 10, 300, 200);
/// for i in 0..30 {
///     RnclCheckBox::new(scroll.content(), &format!("Option {i}")).set_position(8, 8 + i * 24);
/// }
/// scroll.set_content_size(0, 8 + 30 * 24);
/// ```
#[derive(Debug, Clone)]
pub struct RnclScrollView {
    widget: Widget,
    content: RnclPanel,
}

impl RnclScrollView {
    pub const TYPE_NAME: &'static str = "RnclScrollView";
    pub const CONTENT_WIDTH: &'static str = "content_width";
    pub const CONTENT_HEIGHT: &'static str = "content_height";

    /// Erzeugt eine leere Scrollfläche (Standardgröße 240×160).
    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::ScrollArea, parent);
        widget.set_size(240, 160);
        let content = RnclPanel::from_widget(widget.scroll_content());
        Self { widget, content }
    }

    /// Der Container für den scrollbaren Inhalt – Kinder hier anlegen.
    pub fn content(&self) -> &RnclPanel {
        &self.content
    }

    /// Mindestgröße des Inhalts. `0` für eine Richtung bedeutet "an den
    /// sichtbaren Bereich anpassen" (typisch: Breite 0, Höhe = Summe der Zeilen).
    pub fn set_content_size(&self, width: i32, height: i32) {
        self.content.widget().set_minimum_size(width, height);
    }

    /// Die aktuelle Größe des Inhalts.
    pub fn content_size(&self) -> ruf::Size {
        self.content.size()
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent))
        })
        .category("Container")
        .description("Scrollbarer Bereich für viele Komponenten")
    }
}

impl Component for RnclScrollView {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.push(PropertyDescriptor::new(
            Self::CONTENT_WIDTH,
            PropertyType::Int,
            "Mindestbreite des Inhalts",
        ));
        props.push(PropertyDescriptor::new(
            Self::CONTENT_HEIGHT,
            PropertyType::Int,
            "Mindesthöhe des Inhalts",
        ));
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        match name {
            Self::CONTENT_WIDTH => Some(self.content_size().width.into()),
            Self::CONTENT_HEIGHT => Some(self.content_size().height.into()),
            _ => base::get(&self.widget, name),
        }
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            Self::CONTENT_WIDTH => {
                let width = base::expect_int(name, &value)?;
                self.set_content_size(width, self.content_size().height);
            }
            Self::CONTENT_HEIGHT => {
                let height = base::expect_int(name, &value)?;
                self.set_content_size(self.content_size().width, height);
            }
            _ => return base::set(&self.widget, name, value),
        }
        Ok(())
    }
}
