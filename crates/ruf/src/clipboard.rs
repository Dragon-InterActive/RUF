//! Die Zwischenablage des Systems (Text).
//!
//! Lesen ist asynchron (das System liefert den Inhalt später); das Ergebnis
//! kommt im GUI-Thread an.

use crate::backend::{Backend, Platform};

/// Legt Text in die Zwischenablage.
pub fn set_text(text: &str) {
    Platform::clipboard_set(text);
}

/// Liest Text aus der Zwischenablage; `f` bekommt ihn (oder `None`, wenn
/// dort kein Text liegt).
pub fn read_text(f: impl FnOnce(Option<String>) + 'static) {
    Platform::clipboard_read(Box::new(f));
}
