//! Modale Standarddialoge (Meldung, Ja/Nein-Frage, Dateiauswahl) mit den
//! Mitteln des Systems.
//!
//! Alle Funktionen blockieren, bis der Anwender den Dialog schließt, und
//! laufen nur im GUI-Thread. `parent` richtet den Dialog an diesem Fenster aus.
//! Die Beschriftung der Knöpfe folgt der Systemsprache.

use std::path::PathBuf;

use crate::Widget;
use crate::backend::{Backend, MessageKind, Platform};
use crate::core;

fn parent_native(parent: Option<&Widget>) -> Option<crate::backend::Native> {
    parent.and_then(|w| core::native(w.id())).map(|(n, _)| n)
}

fn show(parent: Option<&Widget>, kind: MessageKind, title: &str, text: &str) {
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
    let parent = parent_native(parent);
    core::setter(|| Platform::confirm(parent.as_ref(), title, text))
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
    let parent = parent_native(parent);
    core::setter(|| Platform::save_file(parent.as_ref(), title, name, filters))
}

/// Fragt nach einem Ordner.
pub fn choose_folder(parent: Option<&Widget>, title: &str) -> Option<PathBuf> {
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
