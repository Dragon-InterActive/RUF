//! Daten des Code-Editors ([`crate::WidgetKind::CodeEditor`]).

/// Art einer Zeilenmarke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkKind {
    Error,
    Warning,
    Info,
}

/// Eine Marke am Rand des Code-Editors, z. B. ein Compilerfehler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineMark {
    /// Zeile, ab 1.
    pub line: usize,
    pub kind: MarkKind,
    /// Erscheint als Tooltip über der Marke.
    pub message: String,
}

impl LineMark {
    pub fn new(line: usize, kind: MarkKind, message: &str) -> Self {
        Self {
            line,
            kind,
            message: message.to_owned(),
        }
    }
}
