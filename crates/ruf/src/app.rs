//! Die Anwendung und ihre Event-Schleife.

use crate::backend::{Backend, Platform};
use crate::core;

/// Farbschema der Anwendung.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorScheme {
    /// Unbekannt bzw. "dem System folgen".
    Unknown,
    Light,
    Dark,
}

/// Die native Anwendung.
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
///
/// Läuft auf Linux, Windows und macOS mit GTK 4 – siehe
/// [`toolkit`](crate::toolkit).
pub struct Application {
    name: std::cell::RefCell<String>,
    _not_send: std::marker::PhantomData<*const ()>,
}

impl Application {
    /// Erzeugt die Anwendung.
    ///
    /// # Panics
    ///
    /// Wenn bereits eine Anwendung existiert oder das Toolkit nicht
    /// initialisiert werden kann (z. B. Linux ohne grafische Sitzung).
    pub fn new() -> Self {
        assert!(
            !Self::exists(),
            "ruf: Es darf nur eine Application pro Prozess existieren"
        );
        core::app_init();
        Self {
            name: std::cell::RefCell::new(String::new()),
            _not_send: std::marker::PhantomData,
        }
    }

    /// Wie [`Application::new`]. Die Argumente werden ignoriert; die
    /// Funktion bleibt für Kompatibilität mit älteren RUF-Versionen erhalten.
    pub fn with_args<I, S>(_args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::new()
    }

    /// Gibt es bereits eine laufende Anwendung?
    pub fn exists() -> bool {
        core::app_exists()
    }

    /// Startet die Event-Schleife und blockiert bis [`Application::quit`]
    /// aufgerufen oder das letzte Fenster geschlossen wird. Liefert den Exit-Code.
    pub fn exec(&self) -> i32 {
        core::run();
        0
    }

    /// Beendet die Event-Schleife (kann aus jedem Callback aufgerufen werden).
    pub fn quit() {
        core::quit();
    }

    /// Arbeitet anstehende Ereignisse ab, ohne zu blockieren – nützlich in
    /// Tests. Erledigt auch ausstehende [`Widget::delete_later`](crate::Widget::delete_later).
    pub fn process_events() {
        core::process_events();
    }

    /// Setzt den Anwendungsnamen.
    pub fn set_name(&self, name: &str) {
        *self.name.borrow_mut() = name.to_owned();
    }

    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }

    /// Das aktuelle Farbschema des Systems bzw. der Anwendung.
    pub fn color_scheme() -> ColorScheme {
        Platform::color_scheme()
    }

    /// Erzwingt ein Farbschema; `ColorScheme::Unknown` folgt wieder dem System.
    /// Liefert `false`, wenn das Backend das nicht kann.
    pub fn set_color_scheme(scheme: ColorScheme) -> bool {
        Platform::set_color_scheme(scheme)
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        core::app_shutdown();
    }
}
