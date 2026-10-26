use ruf::{
    AsWidget, Completion, Component, ComponentInfo, Fold, LineMark, MouseEvent, PropertyDescriptor,
    PropertyError, PropertyValue, Widget, WidgetKind, base,
};

use crate::props::text;

/// Ein Code-Editor (GtkSourceView): Syntax-Hervorhebung, Zeilennummern,
/// aktuelle Zeile, Einrückung mit Leerzeichen, passende Klammern und Marken
/// am Rand für Fehler, Warnungen und Hinweise. Das Farbschema folgt
/// Hell/Dunkel.
#[derive(Debug, Clone)]
pub struct RnclCodeEditor {
    widget: Widget,
}

impl RnclCodeEditor {
    pub const TYPE_NAME: &'static str = "RnclCodeEditor";

    /// Erzeugt einen leeren Editor (Standardgröße 400×300) für Rust.
    pub fn new<P: AsWidget + ?Sized>(parent: &P) -> Self {
        Self::with_parent(Some(parent.as_widget()))
    }

    pub fn with_parent(parent: Option<&Widget>) -> Self {
        let widget = Widget::create(WidgetKind::CodeEditor, parent);
        widget.set_size(400, 300);
        widget.set_language("rust");
        Self { widget }
    }

    pub fn set_text(&self, text: &str) {
        self.widget.set_text(text);
    }

    pub fn text(&self) -> String {
        self.widget.text()
    }

    /// Sprache für die Hervorhebung: `"rust"`, `"toml"`, `"json"`, `"markdown"` …
    /// (leer = keine).
    pub fn set_language(&self, language: &str) {
        self.widget.set_language(language);
    }

    pub fn language(&self) -> String {
        self.widget.language()
    }

    pub fn set_read_only(&self, read_only: bool) {
        self.widget.set_read_only(read_only);
    }

    pub fn is_read_only(&self) -> bool {
        self.widget.is_read_only()
    }

    /// Marken am Rand; ersetzt die bisherigen.
    pub fn set_line_marks(&self, marks: Vec<LineMark>) {
        self.widget.set_line_marks(marks);
    }

    pub fn line_marks(&self) -> Vec<LineMark> {
        self.widget.line_marks()
    }

    /// Setzt den Cursor an den Anfang der Zeile `line` (ab 1) und scrollt hin.
    pub fn goto_line(&self, line: usize) {
        self.widget.goto_line(line.saturating_sub(1) as i32);
    }

    /// Faltbare Bereiche (Klammerpaare über mehr als zwei Zeilen) mit
    /// Zustand; per Klick auf den Pfeil am Rand faltet der Nutzer selbst.
    pub fn folds(&self) -> Vec<Fold> {
        self.widget.folds()
    }

    /// Faltet den Bereich, der in `line` (ab 1) beginnt, bzw. klappt ihn auf.
    pub fn set_folded(&self, line: usize, folded: bool) {
        self.widget.set_folded(line, folded);
    }

    /// Faltet alle Bereiche bzw. klappt alle auf.
    pub fn set_all_folded(&self, folded: bool) {
        self.widget.set_all_folded(folded);
    }

    /// Cursor: Zeile ab 1, Spalte in Zeichen ab 0.
    pub fn cursor(&self) -> (usize, usize) {
        self.widget.cursor()
    }

    pub fn set_cursor(&self, line: usize, column: usize) {
        self.widget.set_cursor(line, column);
    }

    /// Zeigt Vorschläge am Cursor; die Wahl ersetzt das Wort vor dem Cursor.
    /// Eine leere Liste schließt die Anzeige.
    pub fn show_completions(&self, items: &[Completion]) {
        self.widget.show_completions(items);
    }

    /// Ersetzt das Wort vor dem Cursor (wie die Wahl eines Vorschlags).
    pub fn apply_completion(&self, text: &str) {
        self.widget.apply_completion(text);
    }

    /// Mausklicks (gemeldet beim Loslassen, der Cursor steht dann schon) –
    /// z. B. Strg+Klick für „Gehe zu Definition“.
    pub fn on_mouse(&self, f: impl FnMut(MouseEvent) + 'static) {
        self.widget.on_mouse(f);
    }

    /// Wird bei jeder Textänderung aufgerufen (auch per Code).
    pub fn on_text_changed(&self, f: impl FnMut(String) + 'static) {
        self.widget.on_text_changed(f);
    }

    pub fn info() -> ComponentInfo {
        ComponentInfo::new(Self::TYPE_NAME, |parent| {
            Box::new(Self::with_parent(parent))
        })
        .category("Eingabe")
        .description("Code-Editor mit Hervorhebung, Zeilennummern und Marken")
    }
}

impl Component for RnclCodeEditor {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn type_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn properties(&self) -> Vec<PropertyDescriptor> {
        let mut props = base::descriptors();
        props.push(text::descriptor("Quelltext"));
        props
    }

    fn property(&self, name: &str) -> Option<PropertyValue> {
        text::get(&self.widget, name).or_else(|| base::get(&self.widget, name))
    }

    fn set_property(&mut self, name: &str, value: PropertyValue) -> Result<(), PropertyError> {
        match name {
            text::TEXT => {
                self.set_text(base::expect_text(name, &value)?);
                Ok(())
            }
            _ => base::set(&self.widget, name, value),
        }
    }
}
