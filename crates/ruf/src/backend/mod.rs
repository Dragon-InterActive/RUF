//! Die System-Ebene: GTK 4 (`gtk4-rs`) auf allen Betriebssystemen.
//!
//! | System | Backend | Modul |
//! |---|---|---|
//! | Linux, BSD, Windows, macOS | GTK 4 (`gtk4-rs`) | [`gtk`] |
//!
//! Das Backend implementiert [`Backend`]. Die Schicht bleibt, obwohl es nur
//! noch ein Backend gibt: ein anderes Toolkit ließe sich später wieder
//! einsetzen, ohne Kern und Komponenten anzufassen.
//!
//! # Aufgabenteilung
//!
//! Das Backend ist bewusst "dumm": es erzeugt native Steuerelemente, setzt
//! und liest deren Zustand und meldet Benutzeraktionen über
//! [`crate::core::notify`] bzw. [`crate::core::close_requested`]. Alles
//! andere – Signale bei Änderungen per Code, Ausschließlichkeit von
//! Optionsfeldern, Combobox-Semantik, Geometrie, Callbacks, Panik-Schutz –
//! liegt in [`crate::core`].

use std::path::{Path, PathBuf};

use crate::app::ColorScheme;
use crate::canvas::Shape;
use crate::code::{Completion, Fold, LineMark};
use crate::core::{Id, WidgetKind};
use crate::dialog::FileFilter;
use crate::geometry::{Point, Rect, Size};
use crate::menu::{Menu, MenuBar};
use crate::style::{Color, Style};
use crate::text::{KeySpec, Span};
use crate::tree::TreeRow;

mod gtk;
pub(crate) use self::gtk::Gtk as Platform;

/// Der native Verweis des aktiven Backends.
pub(crate) type Native = <Platform as Backend>::Native;

/// Art einer Meldung (`ruf::dialog`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Information,
    Warning,
    Error,
}

/// Der Vertrag eines Backends. Alle Funktionen laufen im GUI-Thread – außer
/// [`Backend::post`], das aus jedem Thread aufgerufen werden darf.
///
/// `kind` wird mitgegeben, wo ein Backend sonst raten müsste, welches
/// Steuerelement hinter einem Verweis steckt.
pub(crate) trait Backend {
    /// Verweis auf ein natives Steuerelement samt Hilfsobjekten. Klone
    /// verweisen auf dasselbe Steuerelement.
    type Native: Clone;

    /// Name des Toolkits, z. B. `"GTK"`.
    const NAME: &'static str;

    /// Version des Toolkits bzw. Betriebssystems zur Laufzeit.
    fn version() -> String;

    // --- Anwendung ------------------------------------------------------------

    fn init();
    /// Event-Schleife bis [`Backend::quit`].
    fn run();
    fn quit();
    /// Anstehende Ereignisse abarbeiten, ohne zu blockieren.
    fn process_events();
    /// Führt `f` im GUI-Thread aus; aus jedem Thread aufrufbar.
    fn post(f: Box<dyn FnOnce() + Send>) -> bool;
    /// Wiederholender Timer; das Backend ruft [`crate::core::timer_fired`].
    fn timer_start(id: u64, msec: u32);
    fn timer_stop(id: u64);
    fn color_scheme() -> ColorScheme;
    fn set_color_scheme(scheme: ColorScheme) -> bool;
    fn message(parent: Option<&Self::Native>, kind: MessageKind, title: &str, text: &str);
    fn confirm(parent: Option<&Self::Native>, title: &str, text: &str) -> bool;
    /// Fragt nach einer Textzeile (`secret`: verdeckt); `None`, wenn abgebrochen.
    fn input(
        parent: Option<&Self::Native>,
        title: &str,
        label: &str,
        text: &str,
        secret: bool,
    ) -> Option<String>;

    // --- Erzeugen und Struktur ------------------------------------------------

    /// Erzeugt ein Steuerelement. `parent` ist ein Container (Fenster,
    /// Panel, Gruppe, Seite, Scroll-Inhalt). Fenster ohne Elternteil
    /// bleiben unsichtbar bis `set_visible(true)`.
    fn create(id: Id, kind: WidgetKind, parent: Option<&Self::Native>) -> Self::Native;
    /// Gibt das Steuerelement samt nativer Kinder frei.
    fn destroy(native: &Self::Native, kind: WidgetKind);
    fn set_parent(native: &Self::Native, kind: WidgetKind, parent: &Self::Native);
    /// Name der nativen Klasse, z. B. `"GtkButton"`.
    fn class_name(native: &Self::Native, kind: WidgetKind) -> String;

    // --- Gemeinsame Eigenschaften ---------------------------------------------

    /// Position relativ zum Elternteil (Fenster: auf dem Bildschirm, soweit
    /// das System es erlaubt) und Größe (Fenster: Innenbereich).
    fn set_geometry(native: &Self::Native, kind: WidgetKind, rect: Rect);
    /// Bevorzugte Größe für `adjust_size`.
    fn preferred_size(native: &Self::Native, kind: WidgetKind) -> Size;
    fn set_visible(native: &Self::Native, kind: WidgetKind, visible: bool);
    fn set_enabled(native: &Self::Native, kind: WidgetKind, enabled: bool);
    fn set_tool_tip(native: &Self::Native, kind: WidgetKind, text: &str);
    /// Mauszeiger über dem Widget (CSS-Name wie `row-resize`; leer = Standard).
    fn set_mouse_cursor(native: &Self::Native, name: &str);
    /// Höhe (Pixel), die der Text eines Textbereichs bei `width` braucht
    /// (ohne Innenabstand), und die Höhe einer Zeile; `None` bei anderen Widgets.
    fn text_height(native: &Self::Native, width: i32) -> Option<(i32, i32)>;
    fn set_style(native: &Self::Native, kind: WidgetKind, style: &Style);
    fn screenshot(native: &Self::Native, kind: WidgetKind, path: &Path) -> bool;
    /// Tastenkürzel am Fenster; das Backend ruft [`crate::core::shortcut_fired`].
    fn add_shortcut(window: &Self::Native, key: &KeySpec, shortcut: u64) -> bool;

    // --- Text -------------------------------------------------------------------

    /// Fenstertitel, Beschriftung, Label (einfaches HTML erlaubt) oder Inhalt.
    fn set_text(native: &Self::Native, kind: WidgetKind, text: &str);
    /// Inhalt bearbeitbarer Steuerelemente (Eingabefeld, Textbereich,
    /// editierbare Combobox).
    fn text(native: &Self::Native, kind: WidgetKind) -> String;
    fn set_placeholder(native: &Self::Native, kind: WidgetKind, text: &str);
    fn set_password(native: &Self::Native, enabled: bool);
    fn set_read_only(native: &Self::Native, kind: WidgetKind, read_only: bool);
    fn set_word_wrap(native: &Self::Native, wrap: bool);

    // --- Schalter -------------------------------------------------------------

    fn set_checked(native: &Self::Native, kind: WidgetKind, checked: bool);
    fn is_checked(native: &Self::Native, kind: WidgetKind) -> bool;

    // --- Einträge (Combobox, Liste) -------------------------------------------

    /// Ersetzt alle Einträge; die Auswahl setzt der Kern danach.
    fn set_items(native: &Self::Native, kind: WidgetKind, items: &[String]);
    fn set_current_index(native: &Self::Native, kind: WidgetKind, index: i32);
    fn current_index(native: &Self::Native, kind: WidgetKind) -> i32;
    /// Combobox mit/ohne freie Eingabe. Muss das Backend dafür ein neues
    /// Steuerelement erzeugen, liefert es den neuen Verweis.
    fn set_editable(native: &Self::Native, editable: bool) -> Option<Self::Native>;

    // --- Registerkarten ---------------------------------------------------------

    fn tab_add(native: &Self::Native, page_id: Id, title: &str) -> Self::Native;
    fn tab_remove(native: &Self::Native, index: i32);
    fn tab_set_title(native: &Self::Native, index: i32, title: &str);

    // --- Wertebereich (Spinbox, Schieberegler, Fortschritt) ----------------------

    fn set_range(native: &Self::Native, kind: WidgetKind, minimum: i32, maximum: i32);
    fn set_value(native: &Self::Native, kind: WidgetKind, value: i32);
    fn value(native: &Self::Native, kind: WidgetKind) -> i32;
    fn set_vertical(native: &Self::Native, kind: WidgetKind, vertical: bool);

    // --- Scrollbereich ----------------------------------------------------------

    /// Der Container im Scrollbereich (wird beim Erzeugen mit angelegt).
    fn scroll_content(native: &Self::Native, content_id: Id) -> Self::Native;
    /// Scrollt einen Scrollbereich, bis `x`/`y` des Inhalts links oben steht.
    fn scroll_to(native: &Self::Native, x: i32, y: i32);
    fn set_content_min_size(native: &Self::Native, size: Size);

    // --- Code-Editor (Textbereich) ---------------------------------------------

    fn set_monospace(native: &Self::Native, kind: WidgetKind, monospace: bool);
    fn set_highlights(native: &Self::Native, spans: &[Span]);
    /// Zeile (ab 0) farbig hinterlegen; `None` entfernt die Markierung.
    fn set_marked_line(native: &Self::Native, line: Option<(usize, Color)>);
    fn goto_line(native: &Self::Native, line: usize);

    // --- Zeichenfläche, Baumansicht, Menüs ---------------------------------------

    /// Ersetzt die Formen einer Zeichenfläche und zeichnet neu. Maus und
    /// Tastatur meldet das Backend als `Notify::Mouse` / `Notify::Key`.
    fn set_shapes(native: &Self::Native, shapes: &[Shape]);
    /// Zeigt die sichtbaren Zeilen einer Baumansicht (Auswahl setzt der Kern
    /// danach über `set_current_index`). Auf-/Zuklappen meldet das Backend
    /// als `Notify::TreeToggled(zeile)`.
    fn set_tree_rows(native: &Self::Native, rows: &[TreeRow]);
    /// Menüleiste eines Fensters; gewählte Einträge (auch per Kürzel) meldet
    /// das Backend als `Notify::Menu(id)` für das Fenster.
    fn set_menu_bar(window: &Self::Native, bar: &MenuBar);
    /// Kontextmenü an `x`/`y` relativ zu `native`; Meldung wie bei der Menüleiste
    /// für `window`.
    fn popup_menu(
        native: &Self::Native,
        kind: WidgetKind,
        window: &Self::Native,
        menu: &Menu,
        x: i32,
        y: i32,
    );

    // --- Code-Editor ---------------------------------------------------------------

    /// Sprache für die Hervorhebung (`"rust"`, `"toml"` …; leer = keine).
    fn set_language(native: &Self::Native, language: &str);
    /// Marken am Rand (ersetzt die bisherigen).
    fn set_line_marks(native: &Self::Native, marks: &[LineMark]);
    /// Faltbare Bereiche mit Zustand (siehe [`crate::code::fold_regions`]).
    fn folds(native: &Self::Native) -> Vec<Fold>;
    /// Faltet den Bereich, der in `line` (ab 1) beginnt, bzw. klappt ihn auf.
    fn set_folded(native: &Self::Native, line: usize, folded: bool);
    /// Cursor: Zeile ab 1, Spalte in Zeichen ab 0.
    fn cursor(native: &Self::Native) -> (usize, usize);
    fn set_cursor(native: &Self::Native, line: usize, column: usize);
    /// Zeigt Vorschläge am Cursor; die Wahl ersetzt das Wort vor dem Cursor
    /// (meldet `Notify::Changed`). Leere Liste schließt die Anzeige.
    fn show_completions(native: &Self::Native, items: &[Completion]);
    /// Ersetzt das Wort vor dem Cursor durch `text` (wie die Wahl eines Vorschlags).
    fn apply_completion(native: &Self::Native, text: &str);

    // --- Dateidialoge (modal) -----------------------------------------------------

    fn open_file(
        parent: Option<&Self::Native>,
        title: &str,
        filters: &[FileFilter],
    ) -> Option<PathBuf>;
    fn save_file(
        parent: Option<&Self::Native>,
        title: &str,
        name: &str,
        filters: &[FileFilter],
    ) -> Option<PathBuf>;
    fn choose_folder(parent: Option<&Self::Native>, title: &str) -> Option<PathBuf>;

    // --- Fensterposition ------------------------------------------------------------

    /// Setzt die Position eines Fensters auf dem Bildschirm; `false`, wenn das
    /// System es nicht erlaubt. Vor dem ersten Anzeigen wird sie vorgemerkt.
    fn set_window_position(window: &Self::Native, x: i32, y: i32) -> bool;
    /// Aktuelle Position eines angezeigten Fensters, falls das System sie nennt.
    fn window_position(window: &Self::Native) -> Option<Point>;
}
