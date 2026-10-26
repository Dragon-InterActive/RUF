use ruf::{
    AsWidget, Component, ComponentInfo, PropertyDescriptor, PropertyError, PropertyType,
    PropertyValue, Widget, WidgetKind, base,
};

use crate::props::text;

/// Ein mehrzeiliges Textfeld (`QPlainTextEdit`) – das klassische "Memo".
#[derive(Debug, Clone)]
pub struct RnclTextArea {
    widget: Widget,
}

impl RnclTextArea {
    pub const TYPE_NAME: &'static str = "RnclTextArea";
    pub const PLACEHOLDER: &'static str = "placeholder";
    pub const READ_ONLY: &'static str = "read_only";
    pub const MONOSPACE: &'static str = "monospace";

    /// Erzeugt ein leeres Textfeld (Standardgröße 240×120).
    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    /// Wie `new`, aber mit optionalem Elternteil (`None` = eigenständiges Fenster).
    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::TextArea, parent);
        widget.set_size(240, 120);
        Self { widget }
    }

    /// Setzt den gesamten Text (Zeilenumbrüche mit `\n`).
    pub fn set_text(&self, text: &str) {
        self.widget.set_text(text);
    }

    pub fn text(&self) -> String {
        self.widget.text()
    }

    /// Hängt eine Zeile an.
    pub fn append_line(&self, line: &str) {
        let current = self.text();
        if current.is_empty() {
            self.set_text(line);
        } else {
            self.set_text(&format!("{current}\n{line}"));
        }
    }

    pub fn set_placeholder(&self, text: &str) {
        self.widget.set_placeholder(text);
    }

    pub fn placeholder(&self) -> String {
        self.widget.placeholder()
    }

    pub fn set_read_only(&self, read_only: bool) {
        self.widget.set_read_only(read_only);
    }

    pub fn is_read_only(&self) -> bool {
        self.widget.is_read_only()
    }

    /// Schrift mit fester Zeichenbreite – für Quelltext.
    /// Lange Zeilen umbrechen statt seitlich zu scrollen.
    pub fn set_word_wrap(&self, wrap: bool) {
        self.widget.set_word_wrap(wrap);
    }

    pub fn is_word_wrap(&self) -> bool {
        self.widget.is_word_wrap()
    }

    pub fn set_monospace(&self, monospace: bool) {
        self.widget.set_monospace(monospace);
    }

    pub fn is_monospace(&self) -> bool {
        self.widget.is_monospace()
    }

    /// Syntax-Hervorhebung: alle Treffer von `pattern` (regulärer Ausdruck) in
    /// `color` einfärben, optional fett. Regeln gelten zeilenweise, spätere
    /// überschreiben frühere. `false` bei ungültigem Muster oder Farbe.
    ///
    /// ```no_run
    /// # use rncl::prelude::*;
    /// # let app = Application::new();
    /// # let window = RnclWindow::new("t");
    /// let code = RnclTextArea::new(&window);
    /// code.set_monospace(true);
    /// code.add_highlight_rule(r"^\s*\[.*\]", "#2e86c1", true); // TOML-Abschnitte
    /// code.add_highlight_rule(r"#.*$", "gray", false);            // Kommentare
    /// ```
    pub fn add_highlight_rule(&self, pattern: &str, color: &str, bold: bool) -> bool {
        self.widget.add_highlight_rule(pattern, color, bold)
    }

    /// Entfernt alle Hervorhebungsregeln.
    pub fn clear_highlight_rules(&self) {
        self.widget.clear_highlight_rules();
    }

    /// Hinterlegt Zeile `line` (ab 0) farbig, z. B. eine Fehlerzeile; `None` entfernt sie.
    pub fn set_marked_line(&self, line: Option<i32>, color: &str) {
        self.widget.set_marked_line(line, color);
    }

    /// Springt an den Anfang von Zeile `line` (ab 0).
    pub fn goto_line(&self, line: i32) {
        self.widget.goto_line(line);
    }

    /// Wird bei jeder Änderung mit dem vollständigen Text aufgerufen.
    pub fn on_text_changed(&self, f: impl FnMut(String) + 'static) {
        self.widget.on_text_changed(f);
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent))
        })
        .category("Eingabe")
        .description("Mehrzeiliges Textfeld (Memo)")
    }
}

impl Component for RnclTextArea {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.push(text::descriptor("Inhalt"));
        props.push(PropertyDescriptor::new(
            Self::PLACEHOLDER,
            PropertyType::Text,
            "Hinweistext bei leerem Feld",
        ));
        props.push(PropertyDescriptor::new(
            Self::READ_ONLY,
            PropertyType::Bool,
            "Nur lesbar",
        ));
        props.push(PropertyDescriptor::new(
            Self::MONOSPACE,
            PropertyType::Bool,
            "Schrift mit fester Zeichenbreite (Code)",
        ));
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        match name {
            Self::PLACEHOLDER => Some(self.placeholder().into()),
            Self::READ_ONLY => Some(self.is_read_only().into()),
            Self::MONOSPACE => Some(self.is_monospace().into()),
            _ => text::get(&self.widget, name).or_else(|| base::get(&self.widget, name)),
        }
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            text::TEXT => self.set_text(base::expect_text(name, &value)?),
            Self::PLACEHOLDER => self.set_placeholder(base::expect_text(name, &value)?),
            Self::READ_ONLY => self.set_read_only(base::expect_bool(name, &value)?),
            Self::MONOSPACE => self.set_monospace(base::expect_bool(name, &value)?),
            _ => return base::set(&self.widget, name, value),
        }
        Ok(())
    }
}
