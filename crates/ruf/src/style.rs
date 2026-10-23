//! Farben und einfache Gestaltung – auf allen Backends gleich.
//!
//! RUF unterstützt eine kleine CSS-Teilmenge, die in GTK-CSS übersetzt wird:
//!
//! | Eigenschaft | Werte |
//! |---|---|
//! | `color` | Textfarbe |
//! | `background` / `background-color` | Hintergrundfarbe |
//! | `font-weight` | `bold` / `normal` (bzw. `700` / `400`) |
//!
//! Farben: `#rgb`, `#rrggbb` oder einer der Namen aus [`Color::NAMES`].
//! Unbekannte Eigenschaften werden ignoriert.
//!
//! ```
//! use ruf::{Color, Style};
//!
//! let style = Style::parse("color: #c0392b; font-weight: bold;");
//! assert_eq!(style.color, Some(Color::rgb(0xc0, 0x39, 0x2b)));
//! assert!(style.bold);
//! ```

use std::fmt;

/// Eine RGB-Farbe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Die bekannten Farbnamen (CSS-Grundfarben plus einige gängige).
    pub const NAMES: &'static [(&'static str, Color)] = &[
        ("black", Color::rgb(0, 0, 0)),
        ("white", Color::rgb(255, 255, 255)),
        ("red", Color::rgb(255, 0, 0)),
        ("darkred", Color::rgb(139, 0, 0)),
        ("green", Color::rgb(0, 128, 0)),
        ("darkgreen", Color::rgb(0, 100, 0)),
        ("blue", Color::rgb(0, 0, 255)),
        ("darkblue", Color::rgb(0, 0, 139)),
        ("yellow", Color::rgb(255, 255, 0)),
        ("orange", Color::rgb(255, 165, 0)),
        ("purple", Color::rgb(128, 0, 128)),
        ("gray", Color::rgb(128, 128, 128)),
        ("grey", Color::rgb(128, 128, 128)),
        ("darkgray", Color::rgb(169, 169, 169)),
        ("lightgray", Color::rgb(211, 211, 211)),
        ("silver", Color::rgb(192, 192, 192)),
        ("navy", Color::rgb(0, 0, 128)),
        ("teal", Color::rgb(0, 128, 128)),
        ("maroon", Color::rgb(128, 0, 0)),
        ("olive", Color::rgb(128, 128, 0)),
        ("cyan", Color::rgb(0, 255, 255)),
        ("magenta", Color::rgb(255, 0, 255)),
    ];

    /// Liest `#rgb`, `#rrggbb` oder einen Farbnamen.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().to_ascii_lowercase();
        if let Some(hex) = text.strip_prefix('#') {
            let digit = |i: usize, len: usize| u8::from_str_radix(hex.get(i..i + len)?, 16).ok();
            return match hex.len() {
                3 => Some(Self::rgb(
                    digit(0, 1)? * 17,
                    digit(1, 1)? * 17,
                    digit(2, 1)? * 17,
                )),
                6 => Some(Self::rgb(digit(0, 2)?, digit(2, 2)?, digit(4, 2)?)),
                _ => None,
            };
        }
        Self::NAMES
            .iter()
            .find(|(name, _)| *name == text)
            .map(|(_, c)| *c)
    }

    /// Anteile 0.0…1.0.
    pub fn to_f32(self) -> (f32, f32, f32) {
        (
            f32::from(self.r) / 255.0,
            f32::from(self.g) / 255.0,
            f32::from(self.b) / 255.0,
        )
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// Die Gestaltung eines Widgets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Style {
    pub color: Option<Color>,
    pub background: Option<Color>,
    pub bold: bool,
}

impl Style {
    /// Liest die CSS-Teilmenge (siehe Moduldokumentation).
    pub fn parse(css: &str) -> Self {
        let mut style = Self::default();
        for declaration in css.split(';') {
            let Some((key, value)) = declaration.split_once(':') else {
                continue;
            };
            let value = value.trim();
            match key.trim().to_ascii_lowercase().as_str() {
                "color" => style.color = Color::parse(value),
                "background" | "background-color" => style.background = Color::parse(value),
                "font-weight" => {
                    style.bold = matches!(value, "bold" | "bolder" | "600" | "700" | "800" | "900")
                }
                _ => {}
            }
        }
        style
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors() {
        assert_eq!(Color::parse("#fff"), Some(Color::rgb(255, 255, 255)));
        assert_eq!(
            Color::parse(" #1E8449 "),
            Some(Color::rgb(0x1e, 0x84, 0x49))
        );
        assert_eq!(Color::parse("Gray"), Some(Color::rgb(128, 128, 128)));
        assert_eq!(Color::parse("#12"), None);
        assert_eq!(Color::parse("keine"), None);
        assert_eq!(Color::rgb(1, 2, 255).to_string(), "#0102ff");
    }

    #[test]
    fn css_subset() {
        let s = Style::parse("color: red; background-color:#000;font-weight: 700; margin: 3px");
        assert_eq!(s.color, Some(Color::rgb(255, 0, 0)));
        assert_eq!(s.background, Some(Color::rgb(0, 0, 0)));
        assert!(s.bold);
        assert!(Style::parse("").is_empty());
    }
}
