//! Die Anwendung und ihr Event-Loop.

use cxx::UniquePtr;
use cxx_qt_lib::QString;

use crate::ffi::ffi;

pub use crate::ffi::ffi::ColorScheme;

/// Die native Anwendung (`QApplication`).
///
/// Muss als **erstes** erzeugt werden – vor jeder Komponente – und lebt bis zum
/// Programmende. Rust gibt lokale Variablen in umgekehrter Reihenfolge frei;
/// wird `Application` also zuerst angelegt, werden alle Fenster vorher abgebaut.
///
/// ```no_run
/// let app = ruf::Application::new();
/// // ... Fenster und Komponenten anlegen ...
/// std::process::exit(app.exec());
/// ```
pub struct Application {
    _inner: UniquePtr<ffi::Application>,
}

impl Application {
    /// Erzeugt die Anwendung mit den Kommandozeilenargumenten des Prozesses.
    ///
    /// # Panics
    ///
    /// Wenn bereits eine Anwendung existiert.
    pub fn new() -> Self {
        Self::with_args(std::env::args())
    }

    /// Erzeugt die Anwendung mit eigenen Argumenten (z. B. `["app", "-platform", "offscreen"]`).
    pub fn with_args<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        assert!(
            !Self::exists(),
            "ruf: Es darf nur eine Application pro Prozess existieren"
        );
        let args: Vec<String> = args.into_iter().map(Into::into).collect();
        Self {
            _inner: ffi::application_new(&args),
        }
    }

    /// Gibt es bereits eine laufende Anwendung?
    pub fn exists() -> bool {
        ffi::application_exists()
    }

    /// Startet den Event-Loop und blockiert bis [`Application::quit`]
    /// aufgerufen oder das letzte Fenster geschlossen wird. Liefert den Exit-Code.
    pub fn exec(&self) -> i32 {
        ffi::application_exec()
    }

    /// Beendet den Event-Loop (kann aus jedem Callback aufgerufen werden).
    pub fn quit() {
        ffi::application_quit();
    }

    /// Arbeitet anstehende Events ab, ohne zu blockieren – nützlich in Tests.
    /// Erledigt auch ausstehende [`Widget::delete_later`](crate::Widget::delete_later).
    pub fn process_events() {
        ffi::application_process_events();
    }

    /// Setzt den Anwendungsnamen (wird u. a. für Einstellungen genutzt).
    pub fn set_name(&self, name: &str) {
        ffi::application_set_name(&QString::from(name));
    }

    pub fn name(&self) -> String {
        ffi::application_name().to_string()
    }

    /// Das aktuelle Farbschema des Systems bzw. der Anwendung (Qt >= 6.5).
    pub fn color_scheme() -> ColorScheme {
        ffi::application_color_scheme()
    }

    /// Erzwingt ein Farbschema; `ColorScheme::Unknown` folgt wieder dem System.
    /// Benötigt Qt >= 6.8 – liefert `false`, wenn das gebundene Qt das nicht kann.
    pub fn set_color_scheme(scheme: ColorScheme) -> bool {
        ffi::application_set_color_scheme(scheme)
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}
