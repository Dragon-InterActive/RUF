use ruf::{
    Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyType, PropertyValue,
    Widget, WidgetKind, base,
};

/// Ein Top-Level-Fenster – der Container für alle anderen Komponenten.
///
/// Wird das (besitzende) `RnclWindow` gedroppt, schließt sich das Fenster und
/// alle enthaltenen Komponenten werden mit freigegeben.
#[derive(Debug, Clone)]
pub struct RnclWindow {
    widget: Widget,
}

impl RnclWindow {
    pub const TYPE_NAME: &'static str = "RnclWindow";
    pub const TITLE: &'static str = "title";

    /// Erzeugt ein (noch unsichtbares) Fenster mit Titel und Standardgröße 640×480.
    pub fn new(title: &str) -> Self {
        let widget = Widget::new(WidgetKind::Window);
        widget.set_text(title);
        widget.set_size(640, 480);
        Self { widget }
    }

    pub fn set_title(&self, title: &str) {
        self.widget.set_text(title);
    }

    pub fn title(&self) -> String {
        self.widget.text()
    }

    /// Wird bei jeder Größenänderung mit neuer Breite und Höhe aufgerufen –
    /// hier die Kinder neu anordnen.
    pub fn on_resized(&self, f: impl FnMut(i32, i32) + 'static) {
        self.widget.on_resized(f);
    }

    /// Wird vor dem Schließen aufgerufen; `false` hält das Fenster offen.
    pub fn on_close_requested(&self, f: impl FnMut() -> bool + 'static) {
        self.widget.on_close_requested(f);
    }

    /// Tastenkürzel für dieses Fenster, z. B. `"Ctrl+S"`. `false` bei ungültiger Folge.
    pub fn add_shortcut(&self, sequence: &str, f: impl FnMut() + 'static) -> bool {
        self.widget.add_shortcut(sequence, f)
    }

    /// Schließt das Fenster. Ist es das letzte, endet `Application::exec`.
    pub fn close(&self) {
        self.widget.close();
    }

    pub fn info() -> ComponentInfo {
        // Fenster sind immer Top-Level; ein übergebenes Elternteil wird ignoriert.
        ComponentInfo::new(Self::TYPE_NAME, |_| Box::new(Self::new("Fenster")))
            .category("Container")
            .description("Top-Level-Fenster, enthält andere Komponenten")
            .container(true)
    }
}

impl Component for RnclWindow {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.push(PropertyDescriptor::new(
            Self::TITLE,
            PropertyType::Text,
            "Fenstertitel",
        ));
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        match name {
            Self::TITLE => Some(self.title().into()),
            _ => base::get(&self.widget, name),
        }
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            Self::TITLE => self.set_title(base::expect_text(name, &value)?),
            _ => return base::set(&self.widget, name, value),
        }
        Ok(())
    }
}
