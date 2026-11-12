//! Modale Standarddialoge (Meldung, Ja/Nein-Frage, Texteingabe, Passwort, Dateiauswahl) mit den
//! Mitteln des Systems.
//!
//! Alle Funktionen blockieren, bis der Anwender den Dialog schließt, und
//! laufen nur im GUI-Thread. `parent` richtet den Dialog an diesem Fenster aus.
//! Die Beschriftung der Knöpfe folgt der Systemsprache. Datei- und
//! Ordnerdialoge beginnen im Standardordner ([`set_default_folder`]).
//!
//! Für Tests und Automatisierung beantwortet [`set_automatic`] alle Dialoge
//! sofort, ohne Fenster (die Frage geht nach stderr) – so bleibt ein Test
//! ohne Bildschirm nicht an einem unsichtbaren Dialog hängen.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

thread_local! {
    static DEFAULT_FOLDER: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static AUTOMATIC: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// Beantwortet alle Dialoge automatisch: `Some(true)` = „Ja“ bei Fragen,
/// `Some(false)` = „Nein“; Meldungen werden nur nach stderr geschrieben,
/// Eingabe- und Dateidialoge liefern `None`. `None` schaltet zurück.
pub fn set_automatic(answer: Option<bool>) {
    AUTOMATIC.with(|a| a.set(answer));
}

/// Die automatische Antwort, falls eingeschaltet (und meldet die Frage).
fn automatic(kind: &str, title: &str, text: &str) -> Option<bool> {
    let answer = AUTOMATIC.with(std::cell::Cell::get)?;
    eprintln!("ruf: Dialog ({kind}, automatisch {answer}): {title}: {text}");
    Some(answer)
}

/// Ordner, in dem Datei- und Ordnerdialoge beginnen (z. B. das
/// Arbeitsverzeichnis aus den Einstellungen); `None` = Vorgabe des Systems.
/// Ein nicht vorhandener Ordner wird übergangen.
pub fn set_default_folder(folder: Option<&Path>) {
    DEFAULT_FOLDER.with(|f| *f.borrow_mut() = folder.map(Path::to_path_buf));
}

/// Der Standardordner der Dateidialoge, falls gesetzt und vorhanden.
pub fn default_folder() -> Option<PathBuf> {
    DEFAULT_FOLDER
        .with(|f| f.borrow().clone())
        .filter(|f| f.is_dir())
}

use crate::Widget;
use crate::backend::{Backend, MessageKind, Platform};
use crate::core;

fn parent_native(parent: Option<&Widget>) -> Option<crate::backend::Native> {
    parent.and_then(|w| core::native(w.id())).map(|(n, _)| n)
}

fn show(parent: Option<&Widget>, kind: MessageKind, title: &str, text: &str) {
    if automatic("Meldung", title, text).is_some() {
        return;
    }
    let parent = parent_native(parent);
    core::setter(|| Platform::message(parent.as_ref(), kind, title, text));
}

/// Zeigt einen Hinweis.
pub fn information(parent: Option<&Widget>, title: &str, text: &str) {
    show(parent, MessageKind::Information, title, text);
}

/// Zeigt eine Warnung.
pub fn warning(parent: Option<&Widget>, title: &str, text: &str) {
    show(parent, MessageKind::Warning, title, text);
}

/// Zeigt eine Fehlermeldung.
pub fn error(parent: Option<&Widget>, title: &str, text: &str) {
    show(parent, MessageKind::Error, title, text);
}

/// Stellt eine Ja/Nein-Frage; `true` bei „Ja“.
pub fn confirm(parent: Option<&Widget>, title: &str, text: &str) -> bool {
    if let Some(answer) = automatic("Frage", title, text) {
        return answer;
    }
    let parent = parent_native(parent);
    core::setter(|| Platform::confirm(parent.as_ref(), title, text))
}

/// Fragt nach einer Textzeile (`text` ist der Vorschlag, ausgewählt bis
/// zum letzten Punkt – passend für Dateinamen); `None` bei Abbruch.
pub fn input(parent: Option<&Widget>, title: &str, label: &str, text: &str) -> Option<String> {
    if automatic("Eingabe", title, label).is_some() {
        return None;
    }
    let parent = parent_native(parent);
    core::setter(|| Platform::input(parent.as_ref(), title, label, text, false))
}

/// Fragt verdeckt nach einem Passwort oder einer Passphrase; `None` bei Abbruch.
pub fn password(parent: Option<&Widget>, title: &str, label: &str) -> Option<String> {
    if automatic("Passwort", title, label).is_some() {
        return None;
    }
    let parent = parent_native(parent);
    core::setter(|| Platform::input(parent.as_ref(), title, label, "", true))
}

/// Ein Dateityp-Filter der Dateiauswahl, z. B. `FileFilter::new("Rust", &["*.rs"])`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFilter {
    pub name: String,
    /// Muster wie `*.rs`.
    pub patterns: Vec<String>,
}

impl FileFilter {
    pub fn new(name: &str, patterns: &[&str]) -> Self {
        Self {
            name: name.to_owned(),
            patterns: patterns.iter().map(|p| (*p).to_owned()).collect(),
        }
    }
}

/// Fragt nach einer vorhandenen Datei; `None`, wenn abgebrochen.
pub fn open_file(parent: Option<&Widget>, title: &str, filters: &[FileFilter]) -> Option<PathBuf> {
    if automatic("Datei öffnen", title, "").is_some() {
        return None;
    }
    let parent = parent_native(parent);
    core::setter(|| Platform::open_file(parent.as_ref(), title, filters))
}

/// Fragt nach einem Dateinamen zum Speichern (`name` ist der Vorschlag).
pub fn save_file(
    parent: Option<&Widget>,
    title: &str,
    name: &str,
    filters: &[FileFilter],
) -> Option<PathBuf> {
    if automatic("Speichern unter", title, name).is_some() {
        return None;
    }
    let parent = parent_native(parent);
    core::setter(|| Platform::save_file(parent.as_ref(), title, name, filters))
}

/// Fragt nach einem Ordner.
pub fn choose_folder(parent: Option<&Widget>, title: &str) -> Option<PathBuf> {
    if automatic("Ordner", title, "").is_some() {
        return None;
    }
    let parent = parent_native(parent);
    core::setter(|| Platform::choose_folder(parent.as_ref(), title))
}

/// Sprache der Knopfbeschriftung: Deutsch, wenn die Systemsprache Deutsch ist.
#[allow(dead_code)]
pub(crate) fn german() -> bool {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty()))
        .is_some_and(|l| l.starts_with("de"))
}
