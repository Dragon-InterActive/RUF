//! Text-Hilfen, die alle Backends teilen: Tastenkürzel, einfaches HTML in
//! Labels und die Syntax-Hervorhebung des Code-Editors.

use regex::Regex;

use crate::style::Color;

// --- Tastenkürzel --------------------------------------------------------------

/// Eine Taste ohne Modifizierer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Buchstabe oder Ziffer (immer klein bzw. als Ziffer).
    Char(char),
    /// Funktionstaste F1…F24.
    F(u8),
    Enter,
    Escape,
    Tab,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Left,
    Right,
    Up,
    Down,
    Backspace,
    Space,
}

/// Ein Tastenkürzel wie `Ctrl+S`, `Ctrl+Shift+Z` oder `F5`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeySpec {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    /// macOS: Befehlstaste. Unter macOS gilt `Ctrl+…` ebenfalls als `Cmd+…`.
    pub meta: bool,
    pub key: Key,
}

impl KeySpec {
    /// Liest `"Ctrl+S"`, `"Strg+S"`, `"Shift+F3"`, `"Alt+Enter"`, `"Cmd+Q"`.
    pub fn parse(text: &str) -> Option<Self> {
        let mut spec = Self {
            ctrl: false,
            shift: false,
            alt: false,
            meta: false,
            key: Key::Escape,
        };
        let mut key = None;
        for part in text.split('+').map(str::trim) {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" | "strg" => spec.ctrl = true,
                "shift" | "umschalt" => spec.shift = true,
                "alt" => spec.alt = true,
                "cmd" | "meta" | "command" | "super" => spec.meta = true,
                "" => return None,
                other => {
                    if key.is_some() {
                        return None;
                    }
                    key = Some(parse_key(other)?);
                }
            }
        }
        spec.key = key?;
        Some(spec)
    }
}

fn parse_key(name: &str) -> Option<Key> {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.clone().next())
        && (c.is_ascii_alphanumeric() || c.is_ascii_punctuation())
    {
        return Some(Key::Char(c.to_ascii_lowercase()));
    }
    if let Some(n) = name.strip_prefix('f')
        && let Ok(n) = n.parse::<u8>()
        && (1..=24).contains(&n)
    {
        return Some(Key::F(n));
    }
    Some(match name {
        "comma" | "komma" => Key::Char(','),
        "enter" | "return" => Key::Enter,
        "esc" | "escape" => Key::Escape,
        "tab" => Key::Tab,
        "del" | "delete" | "entf" => Key::Delete,
        "ins" | "insert" | "einfg" => Key::Insert,
        "home" | "pos1" => Key::Home,
        "end" | "ende" => Key::End,
        "pgup" | "pageup" => Key::PageUp,
        "pgdown" | "pagedown" => Key::PageDown,
        "left" => Key::Left,
        "right" => Key::Right,
        "up" => Key::Up,
        "down" => Key::Down,
        "backspace" => Key::Backspace,
        "space" => Key::Space,
        _ => return None,
    })
}

// --- Einfaches HTML in Labels --------------------------------------------------

/// Enthält der Text Auszeichnungen (`<b>`, `<br>` …)?
pub fn looks_like_markup(text: &str) -> bool {
    text.contains('<') && text.contains('>')
}

/// Wandelt einfaches HTML in Pango-Markup (GTK): `<br>` → Zeilenumbruch,
/// `<b> <i> <u> <s> <small> <big> <tt> <sub> <sup>` bleiben, andere Tags
/// fallen weg, `&` wird maskiert.
pub fn markup_to_pango(text: &str) -> String {
    const KEEP: &[&str] = &["b", "i", "u", "s", "small", "big", "tt", "sub", "sup"];
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        out.push_str(&escape_amp(&rest[..start]));
        let Some(end) = rest[start..].find('>') else {
            out.push_str(&escape_amp(&rest[start..]).replace('<', "&lt;"));
            rest = "";
            break;
        };
        let raw = &rest[start + 1..start + end];
        let tag = raw.trim().trim_end_matches('/').trim().to_ascii_lowercase();
        let name = tag.trim_start_matches('/');
        if tag.starts_with("br") {
            out.push('\n');
        } else if KEEP.contains(&name) {
            out.push('<');
            out.push_str(&tag);
            out.push('>');
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(&escape_amp(rest));
    out
}

/// Maskiert `&`, lässt aber vorhandene Entitäten (`&lt;` …) stehen.
fn escape_amp(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (i, c) in text.char_indices() {
        if c == '&' {
            let tail = &text[i..];
            let entity = tail.find(';').is_some_and(|end| {
                end > 1
                    && end < 8
                    && tail[1..end]
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '#')
            });
            out.push_str(if entity { "&" } else { "&amp;" });
        } else {
            out.push(c);
        }
    }
    out
}

// --- Syntax-Hervorhebung -------------------------------------------------------

/// Eine Hervorhebungsregel des Code-Editors.
#[derive(Debug, Clone)]
pub struct HighlightRule {
    pub pattern: Regex,
    pub color: Color,
    pub bold: bool,
}

/// Ein einzufärbender Bereich in Zeichen ab Textanfang; Zeilenumbrüche
/// zählen als ein Zeichen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub len: usize,
    pub color: Color,
    pub bold: bool,
}

/// Wendet die Regeln zeilenweise an. Spätere Regeln überschreiben frühere:
/// das Ergebnis ist eine überschneidungsfreie Liste in Textreihenfolge.
///
/// Hat ein Muster eine Gruppe 1, wird nur diese eingefärbt – Ersatz für
/// Lookarounds, die die `regex`-Crate nicht kennt: `^\s*(\w+)\s*=` färbt nur
/// den Schlüssel, nicht das Gleichheitszeichen.
pub fn highlight(text: &str, rules: &[HighlightRule]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut line_chars = 0usize;
    for line in text.split('\n') {
        // Formatierung je Byte der Zeile: Index der gewinnenden Regel.
        let mut owner: Vec<Option<usize>> = vec![None; line.len()];
        for (index, rule) in rules.iter().enumerate() {
            for caps in rule.pattern.captures_iter(line) {
                let Some(m) = caps.get(1).or_else(|| caps.get(0)) else {
                    continue;
                };
                for slot in &mut owner[m.start()..m.end()] {
                    *slot = Some(index);
                }
            }
        }
        // Zu Bereichen zusammenfassen, Zeichenpositionen mitzählen.
        let mut chars = line_chars;
        let mut current: Option<(usize, Span)> = None;
        for (byte, _) in line.char_indices() {
            let rule = owner[byte];
            match (&mut current, rule) {
                (Some((r, span)), Some(n)) if *r == n => span.len += 1,
                _ => {
                    if let Some((_, span)) = current.take() {
                        spans.push(span);
                    }
                    if let Some(n) = rule {
                        current = Some((
                            n,
                            Span {
                                start: chars,
                                len: 1,
                                color: rules[n].color,
                                bold: rules[n].bold,
                            },
                        ));
                    }
                }
            }
            chars += 1;
        }
        if let Some((_, span)) = current {
            spans.push(span);
        }
        line_chars = chars + 1;
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_specs() {
        let k = KeySpec::parse("Ctrl+S").unwrap();
        assert!(k.ctrl && !k.shift);
        assert_eq!(k.key, Key::Char('s'));
        assert_eq!(KeySpec::parse("F5").unwrap().key, Key::F(5));
        assert!(KeySpec::parse("Strg+Shift+Z").unwrap().shift);
        assert!(KeySpec::parse("").is_none());
        assert!(KeySpec::parse("Ctrl+").is_none());
        assert_eq!(KeySpec::parse("Ctrl+,").unwrap().key, Key::Char(','));
        assert_eq!(KeySpec::parse("Ctrl+Comma").unwrap().key, Key::Char(','));
        assert!(KeySpec::parse("Ctrl+S+T").is_none());
        assert!(KeySpec::parse("Hyper+X").is_none());
    }

    #[test]
    fn markup() {
        assert_eq!(
            markup_to_pango("<b>serde</b> – A & B<br><small>x</small><a href='y'>z</a>"),
            "<b>serde</b> – A &amp; B\n<small>x</small>z"
        );
        assert_eq!(markup_to_pango("&lt;b&gt;"), "&lt;b&gt;");
    }

    #[test]
    fn highlighting() {
        let rules = vec![
            HighlightRule {
                pattern: Regex::new(r"\w+").unwrap(),
                color: Color::rgb(1, 0, 0),
                bold: false,
            },
            HighlightRule {
                pattern: Regex::new("#.*$").unwrap(),
                color: Color::rgb(0, 1, 0),
                bold: true,
            },
        ];
        let spans = highlight("ab #c\nä x", &rules);
        assert_eq!(spans.len(), 4);
        assert_eq!((spans[0].start, spans[0].len), (0, 2));
        assert_eq!((spans[1].start, spans[1].len, spans[1].bold), (3, 2, true));
        // Zweite Zeile: Zeilenumbruch zählt als ein Zeichen.
        assert_eq!((spans[2].start, spans[2].len), (6, 1));
        assert_eq!(spans[3].start, 8);

        // Gruppe 1 statt Lookahead.
        let key = vec![HighlightRule {
            pattern: Regex::new(r"^\s*(\w+)\s*=").unwrap(),
            color: Color::rgb(0, 0, 1),
            bold: false,
        }];
        let spans = highlight("  name = 1", &key);
        assert_eq!((spans[0].start, spans[0].len), (2, 4));
    }
}
