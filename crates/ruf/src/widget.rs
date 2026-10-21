//! [`Widget`]: der sichere Rust-Verweis auf ein natives Qt-Widget.

use cxx::UniquePtr;
use cxx_qt_lib::QString;

use crate::Application;
use crate::ffi::{
    RufBoolCallback, RufCallback, RufCloseCallback, RufIntCallback, RufSizeCallback,
    RufStringCallback, ffi,
};
use crate::geometry::{Point, Rect, Size};

pub use crate::ffi::ffi::WidgetKind;

/// Ein Verweis auf ein natives Widget.
///
/// # Besitz und Lebensdauer
///
/// * Ein Widget **ohne** Elternteil (z. B. ein Fenster) wird freigegeben, wenn
///   der erzeugende `Widget`-Wert gedroppt wird.
/// * Ein Widget **mit** Elternteil gehört – wie in Qt üblich – seinem Elternteil.
///   Den Rust-Wert zu droppen entfernt es nicht aus der Oberfläche.
/// * [`Clone`] erzeugt einen *nicht besitzenden* Verweis, ideal für Closures.
/// * Wurde das Widget von Qt zerstört, werden alle Aufrufe zu No-Ops und
///   Getter liefern Standardwerte ([`Widget::is_alive`] ist dann `false`).
///
/// `Widget` ist bewusst weder `Send` noch `Sync`: Qt-Widgets dürfen nur im
/// GUI-Thread benutzt werden.
pub struct Widget {
    handle: UniquePtr<ffi::WidgetHandle>,
}

impl Widget {
    /// Erzeugt ein Widget ohne Elternteil (z. B. ein Fenster).
    ///
    /// # Panics
    ///
    /// Wenn noch keine [`Application`] existiert.
    pub fn new(kind: WidgetKind) -> Self {
        ensure_application();
        Self {
            handle: ffi::widget_create(kind),
        }
    }

    /// Erzeugt ein Widget als Kind von `parent`.
    ///
    /// # Panics
    ///
    /// Wenn noch keine [`Application`] existiert.
    pub fn new_child(kind: WidgetKind, parent: &Widget) -> Self {
        ensure_application();
        Self {
            handle: ffi::widget_create_child(kind, &parent.handle),
        }
    }

    /// Erzeugt ein Widget – als Kind von `parent` oder, bei `None`, ohne Elternteil.
    pub fn create(kind: WidgetKind, parent: Option<&Widget>) -> Self {
        match parent {
            Some(parent) => Self::new_child(kind, parent),
            None => Self::new(kind),
        }
    }

    /// Erzeugt einen nicht besitzenden Verweis auf ein fremd erzeugtes `QWidget`.
    ///
    /// Damit können eigene RNCL-Komponenten beliebige Qt-Widgets aus einer
    /// eigenen cxx-Bridge in RUF einbinden.
    ///
    /// # Safety
    ///
    /// `widget` muss null sein oder auf ein gültiges `QWidget` zeigen. Danach
    /// überwacht RUF die Lebensdauer selbst (über `QPointer`).
    pub unsafe fn from_raw(widget: *mut ffi::QWidget) -> Self {
        Self {
            // SAFETY: vom Aufrufer zugesichert.
            handle: unsafe { ffi::widget_wrap(widget) },
        }
    }

    /// Ein leerer Verweis (für optionale Elternteile in der Brücke).
    pub(crate) fn null() -> Self {
        // SAFETY: null ist ausdrücklich erlaubt.
        unsafe { Self::from_raw(std::ptr::null_mut()) }
    }

    pub(crate) fn handle(&self) -> &ffi::WidgetHandle {
        &self.handle
    }

    /// Der rohe Zeiger auf das `QWidget` (null, wenn es nicht mehr existiert).
    pub fn as_ptr(&self) -> *mut ffi::QWidget {
        ffi::widget_raw(&self.handle)
    }

    /// Existiert das native Widget noch?
    pub fn is_alive(&self) -> bool {
        ffi::widget_is_alive(&self.handle)
    }

    pub fn kind(&self) -> WidgetKind {
        ffi::widget_kind(&self.handle)
    }

    /// Der Qt-Klassenname, z. B. `"QPushButton"`.
    pub fn class_name(&self) -> String {
        ffi::widget_class_name(&self.handle).to_string()
    }

    /// Hängt das Widget in ein anderes Elternelement um.
    pub fn set_parent(&self, parent: &Widget) {
        ffi::widget_set_parent(&self.handle, &parent.handle);
    }

    // --- Geometrie --------------------------------------------------------

    pub fn set_geometry(&self, rect: Rect) {
        ffi::widget_set_geometry(&self.handle, &rect.into());
    }

    pub fn geometry(&self) -> Rect {
        Rect::from(&ffi::widget_geometry(&self.handle))
    }

    pub fn set_position(&self, x: i32, y: i32) {
        let size = self.size();
        self.set_geometry(Rect::from_parts(Point::new(x, y), size));
    }

    pub fn position(&self) -> Point {
        self.geometry().position()
    }

    pub fn set_size(&self, width: i32, height: i32) {
        let position = self.position();
        self.set_geometry(Rect::from_parts(position, Size::new(width, height)));
    }

    pub fn size(&self) -> Size {
        self.geometry().size()
    }

    pub fn set_minimum_size(&self, width: i32, height: i32) {
        ffi::widget_set_minimum_size(&self.handle, width, height);
    }

    /// Passt die Größe an den Inhalt an (Qt `sizeHint`).
    pub fn adjust_size(&self) {
        ffi::widget_adjust_size(&self.handle);
    }

    // --- Zustand ----------------------------------------------------------

    pub fn set_visible(&self, visible: bool) {
        ffi::widget_set_visible(&self.handle, visible);
    }

    /// Ist das Widget tatsächlich auf dem Bildschirm (inkl. aller Eltern)?
    pub fn is_visible(&self) -> bool {
        ffi::widget_is_visible(&self.handle)
    }

    /// Wurde das Widget explizit versteckt? Kinder eines noch nicht gezeigten
    /// Fensters sind *nicht* versteckt, aber auch noch nicht sichtbar.
    pub fn is_hidden(&self) -> bool {
        ffi::widget_is_hidden(&self.handle)
    }

    pub fn set_enabled(&self, enabled: bool) {
        ffi::widget_set_enabled(&self.handle, enabled);
    }

    pub fn is_enabled(&self) -> bool {
        ffi::widget_is_enabled(&self.handle)
    }

    pub fn set_tool_tip(&self, text: &str) {
        ffi::widget_set_tool_tip(&self.handle, &QString::from(text));
    }

    pub fn tool_tip(&self) -> String {
        ffi::widget_tool_tip(&self.handle).to_string()
    }

    /// Setzt ein Qt-Stylesheet (CSS-ähnlich), z. B. `"color: red;"`.
    pub fn set_style_sheet(&self, css: &str) {
        ffi::widget_set_style_sheet(&self.handle, &QString::from(css));
    }

    pub fn style_sheet(&self) -> String {
        ffi::widget_style_sheet(&self.handle).to_string()
    }

    /// Der Objektname – der Designer nutzt ihn als eindeutigen Bezeichner.
    pub fn set_object_name(&self, name: &str) {
        ffi::widget_set_object_name(&self.handle, &QString::from(name));
    }

    pub fn object_name(&self) -> String {
        ffi::widget_object_name(&self.handle).to_string()
    }

    /// Schließt das Widget (bei Fenstern: wie ein Klick auf "X").
    pub fn close(&self) {
        ffi::widget_close(&self.handle);
    }

    /// Rendert das Widget samt Kindern in eine Bilddatei (Format aus der Endung,
    /// z. B. `.png`). Funktioniert auch offscreen (`-platform offscreen`) –
    /// ideal für Tests, Dokumentation und KI-Assistenten ohne Bildschirm.
    pub fn save_screenshot(&self, path: impl AsRef<std::path::Path>) -> bool {
        let path = path.as_ref().to_string_lossy();
        ffi::widget_save_screenshot(&self.handle, &QString::from(&*path))
    }

    /// Entfernt das Widget samt Kindern aus der Oberfläche und gibt es frei,
    /// sobald der Event-Loop wieder an der Reihe ist. Alle Verweise darauf
    /// werden danach tot ([`Widget::is_alive`] == `false`).
    ///
    /// Gedacht für dynamische Oberflächen, z. B. Listen aus Checkboxen, die
    /// neu aufgebaut werden.
    pub fn delete_later(&self) {
        ffi::widget_delete_later(&self.handle);
    }

    /// Meldet ein Tastenkürzel an (z. B. `"Ctrl+S"`, `"F5"`), das gilt, solange
    /// das Fenster dieses Widgets aktiv ist. `false` bei ungültiger Tastenfolge.
    pub fn add_shortcut(&self, sequence: &str, f: impl FnMut() + 'static) -> bool {
        ffi::widget_add_shortcut(&self.handle, &QString::from(sequence), RufCallback::new(f))
    }

    /// Der Inhalt einer Scrollfläche ([`WidgetKind::ScrollArea`]); bei anderen
    /// Widgets nicht lebendig.
    pub fn scroll_content(&self) -> Widget {
        Widget {
            handle: ffi::widget_scroll_content(&self.handle),
        }
    }

    // --- Code-Editor (Textbereich) -------------------------------------------

    /// Schrift mit fester Zeichenbreite (Systemschrift für Code).
    pub fn set_monospace(&self, monospace: bool) {
        ffi::widget_set_monospace(&self.handle, monospace);
    }

    /// Hat die aktuelle Schrift eine feste Zeichenbreite?
    pub fn is_monospace(&self) -> bool {
        ffi::widget_is_monospace(&self.handle)
    }

    /// Hebt alle Treffer des regulären Ausdrucks `pattern` (Qt/PCRE-Syntax) in
    /// der Farbe `color` (z. B. `"#c0392b"` oder `"darkblue"`) hervor. Regeln
    /// gelten zeilenweise; spätere überschreiben frühere. `false`, wenn das
    /// Widget kein Textbereich oder Muster bzw. Farbe ungültig ist.
    pub fn add_highlight_rule(&self, pattern: &str, color: &str, bold: bool) -> bool {
        ffi::widget_add_highlight_rule(
            &self.handle,
            &QString::from(pattern),
            &QString::from(color),
            bold,
        )
    }

    pub fn clear_highlight_rules(&self) {
        ffi::widget_clear_highlight_rules(&self.handle);
    }

    /// Hinterlegt die Zeile `line` (ab 0) farbig, z. B. für eine Fehlerzeile.
    /// `None` entfernt die Markierung.
    pub fn set_marked_line(&self, line: Option<i32>, color: &str) {
        ffi::widget_set_marked_line(&self.handle, line.unwrap_or(-1), &QString::from(color));
    }

    /// Setzt den Cursor an den Anfang der Zeile `line` (ab 0) und scrollt dorthin.
    pub fn goto_line(&self, line: i32) {
        ffi::widget_goto_line(&self.handle, line);
    }

    // --- Inhalt -----------------------------------------------------------

    /// Fenstertitel bzw. Button-, Label- oder Eingabefeldtext.
    pub fn set_text(&self, text: &str) {
        ffi::widget_set_text(&self.handle, &QString::from(text));
    }

    pub fn text(&self) -> String {
        ffi::widget_text(&self.handle).to_string()
    }

    /// Automatischer Zeilenumbruch eines Labels (Text oben links ausgerichtet).
    pub fn set_word_wrap(&self, wrap: bool) {
        ffi::widget_set_word_wrap(&self.handle, wrap);
    }

    pub fn is_word_wrap(&self) -> bool {
        ffi::widget_is_word_wrap(&self.handle)
    }

    /// Platzhaltertext eines Eingabefelds; bei anderen Widgets ohne Wirkung.
    pub fn set_placeholder(&self, text: &str) {
        ffi::widget_set_placeholder(&self.handle, &QString::from(text));
    }

    pub fn placeholder(&self) -> String {
        ffi::widget_placeholder(&self.handle).to_string()
    }

    /// Verdeckte Eingabe (Passwortfeld); bei anderen Widgets ohne Wirkung.
    pub fn set_password_mode(&self, enabled: bool) {
        ffi::widget_set_password_mode(&self.handle, enabled);
    }

    pub fn is_password_mode(&self) -> bool {
        ffi::widget_is_password_mode(&self.handle)
    }

    /// Häkchen bzw. gedrückter Zustand (macht Buttons dabei "checkable").
    pub fn set_checked(&self, checked: bool) {
        ffi::widget_set_checked(&self.handle, checked);
    }

    pub fn is_checked(&self) -> bool {
        ffi::widget_is_checked(&self.handle)
    }

    /// Löst einen Klick aus wie per Maus (inkl. Umschalten und aller Signale).
    /// Liefert `false`, wenn das Widget kein Button ist.
    pub fn click(&self) -> bool {
        ffi::widget_click(&self.handle)
    }

    /// Nur lesbar (Eingabefeld, Textbereich, Spinbox).
    pub fn set_read_only(&self, read_only: bool) {
        ffi::widget_set_read_only(&self.handle, read_only);
    }

    pub fn is_read_only(&self) -> bool {
        ffi::widget_is_read_only(&self.handle)
    }

    /// Freie Texteingabe in einer Combobox erlauben.
    pub fn set_editable(&self, editable: bool) {
        ffi::widget_set_editable(&self.handle, editable);
    }

    pub fn is_editable(&self) -> bool {
        ffi::widget_is_editable(&self.handle)
    }

    // --- Einträge (Combobox, Liste, Registerkarten) -------------------------
    //
    // Bei Registerkarten sind die "Einträge" die Seitentitel; neue Seiten
    // entstehen über [`Widget::add_tab`].

    pub fn add_item(&self, text: &str) {
        ffi::widget_add_item(&self.handle, &QString::from(text));
    }

    pub fn insert_item(&self, index: i32, text: &str) {
        ffi::widget_insert_item(&self.handle, index, &QString::from(text));
    }

    /// Entfernt einen Eintrag; bei Registerkarten samt Seite und Inhalt.
    pub fn remove_item(&self, index: i32) {
        ffi::widget_remove_item(&self.handle, index);
    }

    pub fn clear_items(&self) {
        ffi::widget_clear_items(&self.handle);
    }

    pub fn item_count(&self) -> i32 {
        ffi::widget_item_count(&self.handle)
    }

    /// Text eines Eintrags (leer bei ungültigem Index).
    pub fn item_text(&self, index: i32) -> String {
        ffi::widget_item_text(&self.handle, index).to_string()
    }

    pub fn set_item_text(&self, index: i32, text: &str) {
        ffi::widget_set_item_text(&self.handle, index, &QString::from(text));
    }

    /// Alle Einträge als Liste.
    pub fn items(&self) -> Vec<String> {
        (0..self.item_count()).map(|i| self.item_text(i)).collect()
    }

    /// Ersetzt alle Einträge (nicht für Registerkarten – dort `add_tab`).
    pub fn set_items<S: AsRef<str>>(&self, items: &[S]) {
        self.clear_items();
        for item in items {
            self.add_item(item.as_ref());
        }
    }

    /// Ausgewählter Eintrag bzw. aktive Seite; `-1` = keine Auswahl.
    pub fn set_current_index(&self, index: i32) {
        ffi::widget_set_current_index(&self.handle, index);
    }

    pub fn current_index(&self) -> i32 {
        ffi::widget_current_index(&self.handle)
    }

    /// Fügt einer Registerkarte eine Seite hinzu und liefert sie als Container.
    /// Bei anderen Widgets ist das Ergebnis nicht lebendig (`is_alive() == false`).
    pub fn add_tab(&self, title: &str) -> Widget {
        Widget {
            handle: ffi::widget_tab_add(&self.handle, &QString::from(title)),
        }
    }

    // --- Wertebereich (Spinbox, Schieberegler, Fortschrittsbalken) ----------

    pub fn set_range(&self, minimum: i32, maximum: i32) {
        ffi::widget_set_range(&self.handle, minimum, maximum);
    }

    pub fn minimum(&self) -> i32 {
        ffi::widget_minimum(&self.handle)
    }

    pub fn maximum(&self) -> i32 {
        ffi::widget_maximum(&self.handle)
    }

    /// Setzt den Wert; Qt begrenzt ihn auf den Wertebereich.
    pub fn set_value(&self, value: i32) {
        ffi::widget_set_value(&self.handle, value);
    }

    pub fn value(&self) -> i32 {
        ffi::widget_value(&self.handle)
    }

    /// Senkrechte statt waagerechte Ausrichtung (Schieberegler, Fortschrittsbalken).
    pub fn set_vertical(&self, vertical: bool) {
        ffi::widget_set_vertical(&self.handle, vertical);
    }

    pub fn is_vertical(&self) -> bool {
        ffi::widget_is_vertical(&self.handle)
    }

    // --- Signale ----------------------------------------------------------
    //
    // Jede Methode liefert `false`, wenn das Widget das Signal nicht besitzt.
    // Closures dürfen andere Widgets verändern – dazu einen `clone()` einfangen.

    /// Klick auf einen Button oder eine Checkbox.
    pub fn on_clicked(&self, f: impl FnMut() + 'static) -> bool {
        ffi::widget_connect_clicked(&self.handle, RufCallback::new(f))
    }

    /// Umschalten eines Buttons oder einer Checkbox.
    pub fn on_toggled(&self, f: impl FnMut(bool) + 'static) -> bool {
        ffi::widget_connect_toggled(&self.handle, RufBoolCallback::new(f))
    }

    /// Textänderung in Eingabefeld, Textbereich oder (editierbarer) Combobox.
    pub fn on_text_changed(&self, f: impl FnMut(String) + 'static) -> bool {
        ffi::widget_connect_text_changed(&self.handle, RufStringCallback::new(f))
    }

    /// Enter-Taste in einem Eingabefeld.
    pub fn on_return_pressed(&self, f: impl FnMut() + 'static) -> bool {
        ffi::widget_connect_return_pressed(&self.handle, RufCallback::new(f))
    }

    /// Wertänderung einer Spinbox, eines Schiebereglers oder Fortschrittsbalkens.
    pub fn on_value_changed(&self, f: impl FnMut(i32) + 'static) -> bool {
        ffi::widget_connect_value_changed(&self.handle, RufIntCallback::new(f))
    }

    /// Auswahlwechsel in Combobox oder Liste bzw. Seitenwechsel einer Registerkarte.
    pub fn on_current_index_changed(&self, f: impl FnMut(i32) + 'static) -> bool {
        ffi::widget_connect_current_index_changed(&self.handle, RufIntCallback::new(f))
    }

    /// Größenänderung (neue Breite, Höhe) – bei jedem Widget, z. B. um Kinder
    /// eines Fensters an dessen Größe anzupassen.
    pub fn on_resized(&self, f: impl FnMut(i32, i32) + 'static) -> bool {
        ffi::widget_connect_resized(&self.handle, RufSizeCallback::new(f))
    }

    /// Wird aufgerufen, bevor ein Fenster geschlossen wird (Klick auf "X",
    /// [`Widget::close`]). Liefert `f` `false`, bleibt das Fenster offen –
    /// z. B. nach der Rückfrage "Ungespeicherte Änderungen verwerfen?".
    pub fn on_close_requested(&self, f: impl FnMut() -> bool + 'static) -> bool {
        ffi::widget_connect_close_requested(&self.handle, RufCloseCallback::new(f))
    }
}

impl Clone for Widget {
    fn clone(&self) -> Self {
        // SAFETY: der Zeiger stammt aus einem gültigen Handle oder ist null.
        unsafe { Self::from_raw(self.as_ptr()) }
    }
}

impl std::fmt::Debug for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Widget")
            .field("class", &self.class_name())
            .field("name", &self.object_name())
            .field("alive", &self.is_alive())
            .finish()
    }
}

/// Alles, was als Elternelement dienen kann: ein [`Widget`] oder jede
/// [`Component`](crate::Component) – auch `dyn Component`.
pub trait AsWidget {
    fn as_widget(&self) -> &Widget;
}

impl AsWidget for Widget {
    fn as_widget(&self) -> &Widget {
        self
    }
}

impl<T: crate::Component + ?Sized> AsWidget for T {
    fn as_widget(&self) -> &Widget {
        self.widget()
    }
}

fn ensure_application() {
    assert!(
        Application::exists(),
        "ruf: Vor dem ersten Widget muss eine ruf::Application erzeugt werden"
    );
}
