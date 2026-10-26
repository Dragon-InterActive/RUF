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

/// Ein Vorschlag der Vervollständigung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    /// Angezeigter Text.
    pub label: String,
    /// Zusatz (z. B. Typ), grau hinter dem Text.
    pub detail: Option<String>,
    /// Ersetzt das Wort vor dem Cursor.
    pub insert: String,
}

/// Ein faltbarer Bereich: Zeile mit der öffnenden bis Zeile mit der
/// schließenden Klammer (ab 1). Gefaltet verschwinden die Zeilen dazwischen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fold {
    pub start: usize,
    pub end: usize,
    pub folded: bool,
}

/// Faltbereiche von Quelltext mit Klammern (`{}`, `[]`, `()`), wie Rust, C
/// oder JSON. Zeichenketten, Zeichen und Kommentare zählen nicht; ein
/// Bereich braucht mindestens eine Zeile zwischen den Klammern. Beginnen
/// mehrere auf derselben Zeile, gilt der äußerste. Sortiert nach `start`.
pub fn fold_regions(text: &str) -> Vec<Fold> {
    let chars: Vec<char> = text.chars().collect();
    let mut regions: Vec<Fold> = Vec::new();
    let mut open: Vec<(char, usize)> = Vec::new();
    let mut line = 1;
    let mut i = 0;
    // Zählt Zeilenumbrüche in übersprungenen Zeichen mit.
    let skip_to = |from: usize, to: usize, line: &mut usize| {
        *line += chars[from..to.min(chars.len())]
            .iter()
            .filter(|c| **c == '\n')
            .count();
        to
    };
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match c {
            '\n' => {
                line += 1;
                i += 1;
            }
            '/' if next == Some('/') => {
                let end = chars[i..]
                    .iter()
                    .position(|c| *c == '\n')
                    .map_or(chars.len(), |p| i + p);
                i = end;
            }
            '/' if next == Some('*') => {
                // Verschachtelte Blockkommentare wie in Rust.
                let mut depth = 0;
                let mut j = i;
                while j < chars.len() {
                    if chars[j] == '/' && chars.get(j + 1) == Some(&'*') {
                        depth += 1;
                        j += 2;
                    } else if chars[j] == '*' && chars.get(j + 1) == Some(&'/') {
                        depth -= 1;
                        j += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        j += 1;
                    }
                }
                i = skip_to(i, j, &mut line);
            }
            'r' | 'b' if !is_ident(i.checked_sub(1).map(|p| chars[p])) => {
                // Roh-Zeichenketten: r"…", r#"…"#, br"…".
                let mut j = i + 1;
                if c == 'b' {
                    if next != Some('r') {
                        i += 1;
                        continue;
                    }
                    j += 1;
                }
                let hashes = chars[j..].iter().take_while(|c| **c == '#').count();
                j += hashes;
                if chars.get(j) != Some(&'"') {
                    i += 1;
                    continue;
                }
                j += 1;
                while j < chars.len() {
                    if chars[j] == '"'
                        && chars[j + 1..]
                            .iter()
                            .take(hashes)
                            .filter(|c| **c == '#')
                            .count()
                            == hashes
                    {
                        j += 1 + hashes;
                        break;
                    }
                    j += 1;
                }
                i = skip_to(i, j, &mut line);
            }
            '"' => {
                let mut j = i + 1;
                while j < chars.len() && chars[j] != '"' {
                    j += if chars[j] == '\\' { 2 } else { 1 };
                }
                i = skip_to(i, j + 1, &mut line);
            }
            '\'' => {
                // Zeichen ('x', '\n') – sonst eine Lebensdauer ('a).
                if next == Some('\\') {
                    let end = chars[i + 2..]
                        .iter()
                        .position(|c| *c == '\'')
                        .map_or(chars.len(), |p| i + 3 + p);
                    i = end;
                } else if chars.get(i + 2) == Some(&'\'') {
                    i += 3;
                } else {
                    i += 1;
                }
            }
            '{' | '[' | '(' => {
                open.push((c, line));
                i += 1;
            }
            '}' | ']' | ')' => {
                let opening = match c {
                    '}' => '{',
                    ']' => '[',
                    _ => '(',
                };
                // Unpassende Klammern (Tippfehler) überspringen statt alles zu verwerfen.
                if let Some(pos) = open.iter().rposition(|(o, _)| *o == opening) {
                    let (_, start) = open[pos];
                    open.truncate(pos);
                    if line >= start + 2 {
                        regions.push(Fold {
                            start,
                            end: line,
                            folded: false,
                        });
                    }
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    regions.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
    regions.dedup_by_key(|r| r.start);
    regions
}

fn is_ident(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// Sprachen, deren Quelltext [`fold_regions`] versteht.
pub fn folds_language(language: &str) -> bool {
    matches!(
        language,
        "rust" | "json" | "c" | "cpp" | "chdr" | "cpphdr" | "js" | "java" | "css" | "go"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn starts_ends(text: &str) -> Vec<(usize, usize)> {
        fold_regions(text)
            .iter()
            .map(|f| (f.start, f.end))
            .collect()
    }

    #[test]
    fn braces_and_nesting() {
        let text =
            "fn main() {\n    if x {\n        a();\n    }\n    let v = [\n        1,\n    ];\n}\n";
        assert_eq!(starts_ends(text), [(1, 8), (2, 4), (5, 7)]);
    }

    #[test]
    fn short_and_same_line() {
        // Zwei Zeilen: nichts zu verbergen. Gleiche Startzeile: der äußere.
        assert_eq!(starts_ends("fn a() {\n}\n"), []);
        assert_eq!(starts_ends("f(vec![\n    1,\n]);\n"), [(1, 3)]);
    }

    #[test]
    fn strings_chars_comments_ignored() {
        let text = concat!(
            "fn f<'a>(s: &'a str) {\n",
            "    let a = \"{\\\"\";\n",
            "    let b = '{';\n",
            "    let c = '\\'';\n",
            "    let d = r#\"}\"# ;\n",
            "    // }\n",
            "    /* { /* } */ */\n",
            "}\n",
        );
        assert_eq!(starts_ends(text), [(1, 8)]);
    }

    #[test]
    fn multi_line_string_counts_lines() {
        let text = "fn f() {\n    let s = \"a\nb\nc\";\n    x();\n}\n";
        assert_eq!(starts_ends(text), [(1, 6)]);
    }

    #[test]
    fn unbalanced_closing_is_skipped() {
        assert_eq!(starts_ends("fn f() {\n    )\n    x();\n}\n"), [(1, 4)]);
    }
}
