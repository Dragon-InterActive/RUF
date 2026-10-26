//! [`Widget`]: der sichere Rust-Verweis auf ein natives Steuerelement.

use crate::backend::{Backend, Platform};
use crate::canvas::{KeyEvent, MouseEvent, Shape};
use crate::code::{Completion, Fold, LineMark};
use crate::core::{self, Id, ItemChange};
use crate::geometry::{Point, Rect, Size};
use crate::menu::{Menu, MenuBar};
use crate::style::Color;
use crate::style::Style;
use crate::text::HighlightRule;
use crate::tree::TreeItem;

pub use crate::core::WidgetKind;

/// Ein Verweis auf ein natives Widget.
///
/// # Besitz und Lebensdauer
///
/// * Ein Widget **ohne** Elternteil (z. B. ein Fenster) wird freigegeben, wenn
///   der erzeugende `Widget`-Wert gedroppt wird.
/// * Ein Widget **mit** Elternteil gehört seinem Elternteil. Den Rust-Wert
///   zu droppen entfernt es nicht aus der Oberfläche.
/// * [`Clone`] erzeugt einen *nicht besitzenden* Verweis, ideal für Closures.
/// * Wurde das Widget zerstört, werden alle Aufrufe zu No-Ops und Getter
///   liefern Standardwerte ([`Widget::is_alive`] ist dann `false`).
///
/// `Widget` ist bewusst weder `Send` noch `Sync`: native Steuerelemente
/// dürfen nur im GUI-Thread benutzt werden. Für Hintergrundarbeit gibt es
/// [`GuiSender`](crate::GuiSender).
pub struct Widget {
    id: Id,
    owned: bool,
    // Nicht Send/Sync: nur im GUI-Thread benutzbar.
    _not_send: std::marker::PhantomData<*const ()>,
}

impl Widget {
    fn wrap(id: Id, owned: bool) -> Self {
        Self {
            id,
            owned,
            _not_send: std::marker::PhantomData,
        }
    }

    /// Erzeugt ein Widget ohne Elternteil (z. B. ein Fenster).
    ///
    /// # Panics
    ///
    /// Wenn noch keine [`Application`](crate::Application) existiert.
    pub fn new(kind: WidgetKind) -> Self {
        ensure_application();
        Self::wrap(core::create(kind, None), true)
    }

    /// Erzeugt ein Widget als Kind von `parent`.
    ///
    /// # Panics
    ///
    /// Wenn noch keine [`Application`](crate::Application) existiert.
    pub fn new_child(kind: WidgetKind, parent: &Widget) -> Self {
        ensure_application();
        Self::wrap(core::create(kind, Some(parent.id)), true)
    }

    /// Erzeugt ein Widget – als Kind von `parent` oder, bei `None`, ohne Elternteil.
    pub fn create(kind: WidgetKind, parent: Option<&Widget>) -> Self {
        match parent {
            Some(parent) => Self::new_child(kind, parent),
            None => Self::new(kind),
        }
    }

    /// Ein Verweis, der auf nichts zeigt (`is_alive() == false`).
    pub fn null() -> Self {
        Self::wrap(0, false)
    }

    /// Die interne Nummer (eindeutig, wird nie wiederverwendet).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Existiert das native Widget noch?
    pub fn is_alive(&self) -> bool {
        core::alive(self.id)
    }

    /// Die Art des Widgets (bei toten Verweisen `Container`).
    pub fn kind(&self) -> WidgetKind {
        core::get(self.id, |n| n.kind).unwrap_or(WidgetKind::Container)
    }

    /// Name der nativen Klasse, z. B. `"GtkButton"`, `"Button"`, `"NSButton"`.
    pub fn class_name(&self) -> String {
        core::native(self.id)
            .map(|(n, k)| Platform::class_name(&n, k))
            .unwrap_or_default()
    }

    /// Der Elternteil (sofern vorhanden und lebendig).
    pub fn parent(&self) -> Option<Widget> {
        core::get(self.id, |n| n.parent)
            .flatten()
            .map(|p| Self::wrap(p, false))
    }

    /// Hängt das Widget in ein anderes Elternelement um.
    pub fn set_parent(&self, parent: &Widget) {
        let (Some((native, kind)), Some((parent_native, parent_kind))) =
            (core::native(self.id), core::native(parent.id))
        else {
            return;
        };
        if !parent_kind.is_container() {
            return;
        }
        let old = core::get(self.id, |n| n.parent).flatten();
        core::setter(|| Platform::set_parent(&native, kind, &parent_native));
        if let Some(old) = old {
            core::update(old, |n| n.children.retain(|c| *c != self.id));
        }
        core::update(self.id, |n| n.parent = Some(parent.id));
        core::update(parent.id, |n| n.children.push(self.id));
        let geometry = self.geometry();
        core::set_geometry(self.id, geometry);
    }

    fn native_call(&self, f: impl FnOnce(&crate::backend::Native, WidgetKind)) {
        if let Some((native, kind)) = core::native(self.id) {
            core::setter(|| f(&native, kind));
        }
    }

    // --- Geometrie --------------------------------------------------------

    pub fn set_geometry(&self, rect: Rect) {
        core::set_geometry(self.id, rect);
    }

    pub fn geometry(&self) -> Rect {
        core::get(self.id, |n| n.geometry).unwrap_or_default()
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

    /// Mindestgröße. Beim Inhalt eines Scrollbereichs bestimmt sie, ab wann
    /// gescrollt wird (0 = folgt dem sichtbaren Bereich).
    pub fn set_minimum_size(&self, width: i32, height: i32) {
        let size = Size::new(width, height);
        // Inhalt eines Scrollbereichs: Mindestgröße am Bereich setzen.
        let scroll = core::get(self.id, |n| n.parent).flatten().and_then(|p| {
            core::get(p, |n| {
                (n.content == Some(self.id)).then(|| n.native.clone())
            })
            .flatten()
        });
        core::update(self.id, |n| n.content_min = size);
        if let Some(native) = scroll {
            core::setter(|| Platform::set_content_min_size(&native, size));
        }
    }

    pub fn minimum_size(&self) -> Size {
        core::get(self.id, |n| n.content_min).unwrap_or_default()
    }

    /// Passt die Größe an den Inhalt an (bevorzugte Größe des Toolkits).
    pub fn adjust_size(&self) {
        if let Some((native, kind)) = core::native(self.id) {
            let size = Platform::preferred_size(&native, kind);
            self.set_size(size.width, size.height);
        }
    }

    // --- Zustand ----------------------------------------------------------

    pub fn set_visible(&self, visible: bool) {
        core::set_visible(self.id, visible);
    }

    /// Ist das Widget tatsächlich sichtbar (inkl. aller Eltern)?
    pub fn is_visible(&self) -> bool {
        core::is_visible(self.id)
    }

    /// Wurde das Widget ausdrücklich ausgeblendet? Kinder eines noch nicht
    /// gezeigten Fensters sind *nicht* ausgeblendet, aber auch nicht sichtbar.
    pub fn is_hidden(&self) -> bool {
        core::get(self.id, |n| n.hidden).unwrap_or(true)
    }

    pub fn set_enabled(&self, enabled: bool) {
        core::update(self.id, |n| n.enabled = enabled);
        self.native_call(|n, k| Platform::set_enabled(n, k, enabled));
    }

    pub fn is_enabled(&self) -> bool {
        core::get(self.id, |n| n.enabled).unwrap_or(false)
    }

    pub fn set_tool_tip(&self, text: &str) {
        core::update(self.id, |n| n.tool_tip = text.to_owned());
        self.native_call(|n, k| Platform::set_tool_tip(n, k, text));
    }

    pub fn tool_tip(&self) -> String {
        core::get(self.id, |n| n.tool_tip.clone()).unwrap_or_default()
    }

    /// Gestaltung per CSS-Teilmenge: `color`, `background(-color)`,
    /// `font-weight` – siehe [`Style`]. Andere Angaben werden ignoriert.
    pub fn set_style_sheet(&self, css: &str) {
        core::update(self.id, |n| n.style_sheet = css.to_owned());
        let style = Style::parse(css);
        self.native_call(|n, k| Platform::set_style(n, k, &style));
    }

    pub fn style_sheet(&self) -> String {
        core::get(self.id, |n| n.style_sheet.clone()).unwrap_or_default()
    }

    /// Der Objektname – der Designer nutzt ihn als eindeutigen Bezeichner.
    pub fn set_object_name(&self, name: &str) {
        core::update(self.id, |n| n.name = name.to_owned());
    }

    pub fn object_name(&self) -> String {
        core::get(self.id, |n| n.name.clone()).unwrap_or_default()
    }

    /// Schließt ein Fenster wie ein Klick auf "X": fragt die
    /// [`on_close_requested`](Self::on_close_requested)-Handler und blendet es
    /// dann aus. Bei anderen Widgets: ausblenden.
    pub fn close(&self) {
        if self.kind() == WidgetKind::Window {
            core::close_requested(self.id);
        } else {
            self.set_visible(false);
        }
    }

    /// Entfernt das Widget samt Kindern aus der Oberfläche und gibt es frei,
    /// sobald die Event-Schleife wieder an der Reihe ist (bzw. beim nächsten
    /// [`Application::process_events`](crate::Application::process_events)).
    /// Alle Verweise darauf werden danach tot.
    pub fn delete_later(&self) {
        core::delete_later(self.id);
    }

    /// Rendert das Widget samt Kindern in eine PNG-Datei – ideal für Tests,
    /// Dokumentation und KI-Assistenten. Liefert `false` bei Fehlern.
    pub fn save_screenshot(&self, path: impl AsRef<std::path::Path>) -> bool {
        match core::native(self.id) {
            Some((native, kind)) => {
                core::setter(|| Platform::screenshot(&native, kind, path.as_ref()))
            }
            None => false,
        }
    }

    /// Meldet ein Tastenkürzel an (z. B. `"Ctrl+S"`, `"F5"`, `"Strg+Shift+Z"`),
    /// das gilt, solange das Fenster dieses Widgets aktiv ist. `false` bei
    /// ungültiger Tastenfolge. Unter macOS gilt `Ctrl` auch als `Cmd`.
    pub fn add_shortcut(&self, sequence: &str, f: impl FnMut() + 'static) -> bool {
        core::add_shortcut(self.id, sequence, f)
    }

    /// Der Inhalt eines Scrollbereichs ([`WidgetKind::ScrollArea`]); bei
    /// anderen Widgets nicht lebendig.
    pub fn scroll_content(&self) -> Widget {
        let content = core::get(self.id, |n| n.content).flatten().unwrap_or(0);
        Self::wrap(content, false)
    }

    // --- Code-Editor (Textbereich) -------------------------------------------

    /// Schrift mit fester Zeichenbreite (Systemschrift für Code).
    pub fn set_monospace(&self, monospace: bool) {
        core::update(self.id, |n| n.monospace = monospace);
        self.native_call(|n, k| Platform::set_monospace(n, k, monospace));
    }

    pub fn is_monospace(&self) -> bool {
        core::get(self.id, |n| n.monospace).unwrap_or(false)
    }

    /// Hebt alle Treffer des regulären Ausdrucks `pattern` (Syntax der
    /// `regex`-Crate) in der Farbe `color` hervor. Regeln gelten zeilenweise;
    /// spätere überschreiben frühere. Hat das Muster eine Gruppe 1, wird nur
    /// diese eingefärbt (Ersatz für Lookarounds). `false`, wenn das Widget
    /// kein Textbereich oder Muster bzw. Farbe ungültig ist.
    pub fn add_highlight_rule(&self, pattern: &str, color: &str, bold: bool) -> bool {
        let (Ok(pattern), Some(color)) = (regex::Regex::new(pattern), Color::parse(color)) else {
            return false;
        };
        if pattern.as_str().is_empty() || self.kind() != WidgetKind::TextArea {
            return false;
        }
        core::update(self.id, |n| {
            n.rules.push(HighlightRule {
                pattern,
                color,
                bold,
            })
        });
        core::rehighlight(self.id, &self.text());
        true
    }

    pub fn clear_highlight_rules(&self) {
        core::update(self.id, |n| n.rules.clear());
        core::rehighlight(self.id, &self.text());
    }

    /// Hinterlegt die Zeile `line` (ab 0) farbig, z. B. eine Fehlerzeile.
    /// `None` entfernt die Markierung.
    pub fn set_marked_line(&self, line: Option<i32>, color: &str) {
        let marked = match (line, Color::parse(color)) {
            (Some(line), Some(color)) if line >= 0 => Some((line as usize, color)),
            _ => None,
        };
        core::update(self.id, |n| n.marked = marked);
        self.native_call(|n, k| {
            if matches!(k, WidgetKind::TextArea | WidgetKind::CodeEditor) {
                Platform::set_marked_line(n, marked);
            }
        });
    }

    /// Setzt den Cursor an den Anfang der Zeile `line` (ab 0) und scrollt dorthin.
    pub fn goto_line(&self, line: i32) {
        if line >= 0 {
            self.native_call(|n, k| {
                if matches!(k, WidgetKind::TextArea | WidgetKind::CodeEditor) {
                    Platform::goto_line(n, line as usize);
                }
            });
        }
    }

    // --- Inhalt -----------------------------------------------------------

    /// Fenstertitel bzw. Button-, Label- oder Eingabefeldtext. Labels
    /// verstehen einfaches HTML (`<b>`, `<i>`, `<br>`, `<small>` …).
    pub fn set_text(&self, text: &str) {
        core::set_text(self.id, text);
    }

    pub fn text(&self) -> String {
        core::current_text(self.id)
    }

    /// Platzhaltertext eines Eingabefelds; bei anderen Widgets ohne Wirkung.
    pub fn set_placeholder(&self, text: &str) {
        core::update(self.id, |n| n.placeholder = text.to_owned());
        self.native_call(|n, k| Platform::set_placeholder(n, k, text));
    }

    pub fn placeholder(&self) -> String {
        core::get(self.id, |n| n.placeholder.clone()).unwrap_or_default()
    }

    /// Verdeckte Eingabe (Passwortfeld); bei anderen Widgets ohne Wirkung.
    pub fn set_password_mode(&self, enabled: bool) {
        if self.kind() == WidgetKind::LineEdit {
            core::update(self.id, |n| n.password = enabled);
            self.native_call(|n, _| Platform::set_password(n, enabled));
        }
    }

    pub fn is_password_mode(&self) -> bool {
        core::get(self.id, |n| n.password).unwrap_or(false)
    }

    /// Automatischer Zeilenumbruch eines Labels.
    pub fn set_word_wrap(&self, wrap: bool) {
        if self.kind() == WidgetKind::Label {
            core::update(self.id, |n| n.word_wrap = wrap);
            self.native_call(|n, _| Platform::set_word_wrap(n, wrap));
        }
    }

    pub fn is_word_wrap(&self) -> bool {
        core::get(self.id, |n| n.word_wrap).unwrap_or(false)
    }

    /// Häkchen bzw. gedrückter Zustand.
    pub fn set_checked(&self, checked: bool) {
        core::set_checked(self.id, checked);
    }

    pub fn is_checked(&self) -> bool {
        core::native(self.id)
            .map(|(n, k)| Platform::is_checked(&n, k))
            .unwrap_or(false)
    }

    /// Löst einen Klick aus wie per Maus (inkl. Umschalten und aller Signale).
    /// Liefert `false`, wenn das Widget kein Button ist.
    pub fn click(&self) -> bool {
        core::click(self.id)
    }

    /// Nur lesbar (Eingabefeld, Textbereich, Spinbox).
    pub fn set_read_only(&self, read_only: bool) {
        core::update(self.id, |n| n.read_only = read_only);
        self.native_call(|n, k| Platform::set_read_only(n, k, read_only));
    }

    pub fn is_read_only(&self) -> bool {
        core::get(self.id, |n| n.read_only).unwrap_or(false)
    }

    /// Freie Texteingabe in einer Combobox erlauben.
    pub fn set_editable(&self, editable: bool) {
        core::set_editable(self.id, editable);
    }

    pub fn is_editable(&self) -> bool {
        core::get(self.id, |n| n.editable).unwrap_or(false)
    }

    // --- Einträge (Combobox, Liste, Registerkarten) -------------------------
    //
    // Bei Registerkarten sind die "Einträge" die Seitentitel; neue Seiten
    // entstehen über [`Widget::add_tab`].

    pub fn add_item(&self, text: &str) {
        core::edit_items(self.id, |items| {
            items.push(text.to_owned());
            ItemChange::Inserted(items.len() as i32 - 1)
        });
    }

    pub fn insert_item(&self, index: i32, text: &str) {
        core::edit_items(self.id, |items| {
            let at = (index.max(0) as usize).min(items.len());
            items.insert(at, text.to_owned());
            ItemChange::Inserted(at as i32)
        });
    }

    /// Entfernt einen Eintrag; bei Registerkarten samt Seite und Inhalt.
    pub fn remove_item(&self, index: i32) {
        if self.kind() == WidgetKind::TabView {
            core::tab_remove(self.id, index);
            return;
        }
        core::edit_items(self.id, |items| match usize::try_from(index) {
            Ok(i) if i < items.len() => {
                items.remove(i);
                ItemChange::Removed(index)
            }
            _ => ItemChange::None,
        });
    }

    pub fn clear_items(&self) {
        if self.kind() == WidgetKind::TabView {
            while self.item_count() > 0 {
                core::tab_remove(self.id, 0);
            }
            return;
        }
        core::edit_items(self.id, |items| {
            items.clear();
            ItemChange::Replaced
        });
    }

    pub fn item_count(&self) -> i32 {
        core::get(self.id, |n| match n.kind {
            WidgetKind::TabView => n.tabs.len() as i32,
            _ => n.items.len() as i32,
        })
        .unwrap_or(0)
    }

    /// Text eines Eintrags (leer bei ungültigem Index).
    pub fn item_text(&self, index: i32) -> String {
        core::get(self.id, |n| {
            let list = if n.kind == WidgetKind::TabView {
                &n.tab_titles
            } else {
                &n.items
            };
            usize::try_from(index)
                .ok()
                .and_then(|i| list.get(i).cloned())
                .unwrap_or_default()
        })
        .unwrap_or_default()
    }

    pub fn set_item_text(&self, index: i32, text: &str) {
        if self.kind() == WidgetKind::TabView {
            core::tab_set_title(self.id, index, text);
            return;
        }
        core::edit_items(self.id, |items| {
            match usize::try_from(index).ok().and_then(|i| items.get_mut(i)) {
                Some(item) => {
                    *item = text.to_owned();
                    ItemChange::Renamed
                }
                None => ItemChange::None,
            }
        });
    }

    /// Alle Einträge als Liste.
    pub fn items(&self) -> Vec<String> {
        (0..self.item_count()).map(|i| self.item_text(i)).collect()
    }

    /// Ersetzt alle Einträge (nicht für Registerkarten – dort `add_tab`).
    pub fn set_items<S: AsRef<str>>(&self, items: &[S]) {
        let items: Vec<String> = items.iter().map(|s| s.as_ref().to_owned()).collect();
        core::edit_items(self.id, move |list| {
            *list = items;
            ItemChange::Replaced
        });
    }

    /// Ausgewählter Eintrag bzw. aktive Seite; `-1` = keine Auswahl.
    pub fn set_current_index(&self, index: i32) {
        core::set_current_index(self.id, index);
    }

    pub fn current_index(&self) -> i32 {
        core::native(self.id)
            .map(|(n, k)| match k {
                WidgetKind::ComboBox | WidgetKind::ListBox | WidgetKind::TabView => {
                    Platform::current_index(&n, k)
                }
                _ => -1,
            })
            .unwrap_or(-1)
    }

    /// Fügt einer Registerkarte eine Seite hinzu und liefert sie als Container.
    /// Bei anderen Widgets ist das Ergebnis nicht lebendig (`is_alive() == false`).
    pub fn add_tab(&self, title: &str) -> Widget {
        Self::wrap(core::tab_add(self.id, title).unwrap_or(0), false)
    }

    // --- Wertebereich (Spinbox, Schieberegler, Fortschrittsbalken) ----------

    pub fn set_range(&self, minimum: i32, maximum: i32) {
        core::set_range(self.id, minimum, maximum);
    }

    pub fn minimum(&self) -> i32 {
        core::get(self.id, |n| n.minimum).unwrap_or(0)
    }

    pub fn maximum(&self) -> i32 {
        core::get(self.id, |n| n.maximum).unwrap_or(0)
    }

    /// Setzt den Wert; er wird auf den Wertebereich begrenzt.
    pub fn set_value(&self, value: i32) {
        core::set_value(self.id, value);
    }

    pub fn value(&self) -> i32 {
        core::native(self.id)
            .map(|(n, k)| match k {
                WidgetKind::SpinBox | WidgetKind::Slider | WidgetKind::ProgressBar => {
                    Platform::value(&n, k)
                }
                _ => 0,
            })
            .unwrap_or(0)
    }

    /// Senkrechte statt waagerechte Ausrichtung (Schieberegler, Fortschrittsbalken).
    pub fn set_vertical(&self, vertical: bool) {
        core::update(self.id, |n| n.vertical = vertical);
        self.native_call(|n, k| Platform::set_vertical(n, k, vertical));
    }

    pub fn is_vertical(&self) -> bool {
        core::get(self.id, |n| n.vertical).unwrap_or(false)
    }

    // --- Zeichenfläche ([`WidgetKind::Canvas`]) ----------------------------

    /// Ersetzt, was die Zeichenfläche malt, und zeichnet neu.
    pub fn set_shapes(&self, shapes: Vec<Shape>) {
        core::set_shapes(self.id, shapes);
    }

    /// Die zuletzt gesetzten Formen.
    pub fn shapes(&self) -> Vec<Shape> {
        core::get(self.id, |n| n.shapes.clone()).unwrap_or_default()
    }

    /// Mausereignisse der Zeichenfläche (Drücken, Loslassen, Bewegen,
    /// Doppelklick). Ein Klick gibt ihr den Tastaturfokus. Der Code-Editor
    /// meldet das Loslassen, die Baumansicht den Rechtsklick (die Zeile ist
    /// dann schon gewählt).
    pub fn on_mouse(&self, f: impl FnMut(MouseEvent) + 'static) -> bool {
        core::connect_mouse(self.id, f)
    }

    /// Tastendrücke, solange die Zeichenfläche den Fokus hat.
    pub fn on_key(&self, f: impl FnMut(KeyEvent) + 'static) -> bool {
        core::connect_key(self.id, f)
    }

    // --- Code-Editor ([`WidgetKind::CodeEditor`]) ----------------------------

    /// Sprache für die Hervorhebung, z. B. `"rust"` oder `"toml"`.
    pub fn set_language(&self, language: &str) {
        core::set_language(self.id, language);
    }

    pub fn language(&self) -> String {
        core::get(self.id, |n| n.language.clone()).unwrap_or_default()
    }

    /// Marken am Rand (Fehler, Warnungen, Hinweise); ersetzt die bisherigen.
    pub fn set_line_marks(&self, marks: Vec<LineMark>) {
        core::set_line_marks(self.id, marks);
    }

    pub fn line_marks(&self) -> Vec<LineMark> {
        core::get(self.id, |n| n.line_marks.clone()).unwrap_or_default()
    }

    /// Faltbare Bereiche eines Code-Editors (Klammerpaare über mehr als
    /// zwei Zeilen) mit ihrem Zustand. Gefaltet wird per Klick auf den Pfeil
    /// am Rand oder mit [`Widget::set_folded`].
    pub fn folds(&self) -> Vec<Fold> {
        match core::native(self.id) {
            Some((native, WidgetKind::CodeEditor)) => Platform::folds(&native),
            _ => Vec::new(),
        }
    }

    /// Faltet den Bereich, der in `line` (ab 1) beginnt, bzw. klappt ihn auf.
    /// Springt der Cursor in einen gefalteten Bereich, klappt er auf.
    pub fn set_folded(&self, line: usize, folded: bool) {
        if let Some((native, WidgetKind::CodeEditor)) = core::native(self.id) {
            Platform::set_folded(&native, line, folded);
        }
    }

    /// Faltet alle Bereiche bzw. klappt alle auf.
    pub fn set_all_folded(&self, folded: bool) {
        // Von innen nach außen, damit verschachtelte Bereiche mitkommen.
        for fold in self.folds().iter().rev() {
            self.set_folded(fold.start, folded);
        }
    }

    /// Cursor eines Code-Editors: Zeile ab 1, Spalte in Zeichen ab 0.
    pub fn cursor(&self) -> (usize, usize) {
        match core::native(self.id) {
            Some((native, WidgetKind::CodeEditor)) => Platform::cursor(&native),
            _ => (1, 0),
        }
    }

    pub fn set_cursor(&self, line: usize, column: usize) {
        if let Some((native, WidgetKind::CodeEditor)) = core::native(self.id) {
            core::setter(|| Platform::set_cursor(&native, line, column));
        }
    }

    /// Ersetzt das Wort vor dem Cursor durch `text` – wie die Wahl eines
    /// Vorschlags (auch für Tests und Automatisierung).
    pub fn apply_completion(&self, text: &str) {
        if let Some((native, WidgetKind::CodeEditor)) = core::native(self.id) {
            Platform::apply_completion(&native, text);
        }
    }

    /// Zeigt Vorschläge am Cursor (leer = schließen). Die Wahl ersetzt das
    /// Wort vor dem Cursor und meldet eine Textänderung.
    pub fn show_completions(&self, items: &[Completion]) {
        if let Some((native, WidgetKind::CodeEditor)) = core::native(self.id) {
            Platform::show_completions(&native, items);
        }
    }

    // --- Baumansicht ([`WidgetKind::TreeView`]) ------------------------------

    /// Ersetzt alle Knoten; eine Auswahl bleibt erhalten, wenn ihr Knoten
    /// sichtbar bleibt.
    pub fn set_tree(&self, items: Vec<TreeItem>) {
        core::set_tree(self.id, items);
    }

    pub fn tree(&self) -> Vec<TreeItem> {
        core::get(self.id, |n| n.tree.clone()).unwrap_or_default()
    }

    /// Kennungen der sichtbaren Zeilen, von oben nach unten.
    pub fn visible_tree_items(&self) -> Vec<String> {
        core::get(self.id, |n| n.tree_rows.clone()).unwrap_or_default()
    }

    /// Klappt einen Knoten auf bzw. zu.
    pub fn toggle_tree_item(&self, id: &str) -> bool {
        core::toggle_tree_item(self.id, id)
    }

    /// Wählt einen sichtbaren Knoten aus (`None` = keine Auswahl).
    pub fn select_tree_item(&self, id: Option<&str>) {
        core::select_tree_item(self.id, id);
    }

    pub fn selected_tree_item(&self) -> Option<String> {
        core::get(self.id, |n| n.tree_selected.clone()).flatten()
    }

    /// Auswahlwechsel (Kennung, leer = keine Auswahl).
    pub fn on_tree_selection_changed(&self, f: impl FnMut(String) + 'static) -> bool {
        core::connect_tree_selected(self.id, f)
    }

    /// Doppelklick bzw. Enter auf einem Knoten.
    pub fn on_tree_item_activated(&self, f: impl FnMut(String) + 'static) -> bool {
        core::connect_tree_activated(self.id, f)
    }

    // --- Menüs -------------------------------------------------------------

    /// Menüleiste eines Fensters (ersetzt eine vorhandene).
    pub fn set_menu_bar(&self, bar: MenuBar) {
        core::set_menu_bar(self.id, bar);
    }

    pub fn menu_bar(&self) -> MenuBar {
        core::get(self.id, |n| n.menu_bar.clone()).unwrap_or_default()
    }

    /// Gewählter Menüeintrag eines Fensters – aus der Menüleiste, einem
    /// Kontextmenü oder per Tastenkürzel.
    pub fn on_menu(&self, f: impl FnMut(String) + 'static) -> bool {
        core::connect_menu(self.id, f)
    }

    /// Zeigt ein Kontextmenü an `x`/`y` (relativ zu diesem Widget). Die
    /// Auswahl meldet `on_menu` seines Fensters.
    pub fn popup_menu(&self, menu: &Menu, x: i32, y: i32) -> bool {
        core::popup_menu(self.id, menu, x, y)
    }

    /// Simuliert ein Mausereignis auf einer Zeichenfläche (Tests, Automatisierung).
    pub fn simulate_mouse(&self, event: MouseEvent) {
        core::notify(self.id, core::Notify::Mouse(event));
    }

    /// Simuliert einen Tastendruck auf einer Zeichenfläche.
    pub fn simulate_key(&self, event: KeyEvent) {
        core::notify(self.id, core::Notify::Key(event));
    }

    /// Simuliert Doppelklick/Enter auf dem ausgewählten Knoten einer Baumansicht.
    pub fn activate_tree_item(&self) {
        core::notify(self.id, core::Notify::Activated);
    }

    /// Simuliert die Wahl eines Menüeintrags (Tests, Automatisierung).
    pub fn activate_menu_item(&self, id: &str) {
        core::notify(self.id, core::Notify::Menu(id.to_owned()));
    }

    // --- Fensterposition ---------------------------------------------------

    /// Setzt die Position eines Fensters auf dem Bildschirm. `false`, wenn das
    /// System Programmen das nicht erlaubt (Wayland).
    pub fn set_window_position(&self, x: i32, y: i32) -> bool {
        core::set_window_position(self.id, x, y)
    }

    /// Position eines angezeigten Fensters, falls das System sie nennt.
    pub fn window_position(&self) -> Option<Point> {
        core::window_position(self.id)
    }

    // --- Signale ----------------------------------------------------------
    //
    // Jede Methode liefert `false`, wenn das Widget das Signal nicht besitzt.
    // Signale feuern auch bei Änderungen per Code. Closures dürfen andere
    // Widgets verändern – dazu einen `clone()` einfangen.

    /// Klick auf einen Button, eine Checkbox oder ein Optionsfeld.
    pub fn on_clicked(&self, f: impl FnMut() + 'static) -> bool {
        core::connect_clicked(self.id, f)
    }

    /// Umschalten einer Checkbox oder eines Optionsfelds.
    pub fn on_toggled(&self, f: impl FnMut(bool) + 'static) -> bool {
        core::connect_toggled(self.id, f)
    }

    /// Textänderung in Eingabefeld, Textbereich oder Combobox.
    pub fn on_text_changed(&self, f: impl FnMut(String) + 'static) -> bool {
        core::connect_text_changed(self.id, f)
    }

    /// Enter-Taste in einem Eingabefeld.
    pub fn on_return_pressed(&self, f: impl FnMut() + 'static) -> bool {
        core::connect_return_pressed(self.id, f)
    }

    /// Wertänderung einer Spinbox, eines Schiebereglers oder Fortschrittsbalkens.
    pub fn on_value_changed(&self, f: impl FnMut(i32) + 'static) -> bool {
        core::connect_value_changed(self.id, f)
    }

    /// Auswahlwechsel in Combobox oder Liste bzw. Seitenwechsel einer Registerkarte.
    pub fn on_current_index_changed(&self, f: impl FnMut(i32) + 'static) -> bool {
        core::connect_index_changed(self.id, f)
    }

    /// Größenänderung (neue Breite, Höhe) – bei jedem Widget, z. B. um Kinder
    /// eines Fensters oder einer Registerkarten-Seite neu anzuordnen.
    pub fn on_resized(&self, f: impl FnMut(i32, i32) + 'static) -> bool {
        core::connect_resized(self.id, f)
    }

    /// Wird aufgerufen, bevor ein Fenster geschlossen wird (Klick auf "X",
    /// [`Widget::close`]). Liefert `f` `false`, bleibt das Fenster offen.
    pub fn on_close_requested(&self, f: impl FnMut() -> bool + 'static) -> bool {
        core::connect_close_requested(self.id, f)
    }
}

impl Clone for Widget {
    fn clone(&self) -> Self {
        Self::wrap(self.id, false)
    }
}

impl Drop for Widget {
    fn drop(&mut self) {
        // Nur Top-Level-Widgets gehören ihrem Rust-Wert; ohne Anwendung wird
        // nichts mehr angefasst (Programmende).
        if self.owned
            && core::app_exists()
            && core::get(self.id, |n| n.parent.is_none()) == Some(true)
        {
            core::destroy(self.id);
        }
    }
}

impl std::fmt::Debug for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Widget")
            .field("id", &self.id)
            .field("kind", &self.kind())
            .field("name", &self.object_name())
            .field("alive", &self.is_alive())
            .finish()
    }
}

impl PartialEq for Widget {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
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
        core::app_exists(),
        "ruf: Vor dem ersten Widget muss eine ruf::Application erzeugt werden"
    );
}
