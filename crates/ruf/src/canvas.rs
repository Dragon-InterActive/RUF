//! Zeichenfläche: Formen, die das Programm vorgibt, und Maus-/Tastatur-
//! ereignisse, die es zurückbekommt.
//!
//! Die Zeichenfläche ([`crate::WidgetKind::Canvas`]) malt eine Liste von
//! [`Shape`]s – mehr nicht. Was gezeichnet wird, entscheidet das Programm
//! (z. B. der Formular-Designer von Freya TD aus seiner Zeichenliste).

use crate::geometry::{Point, Rect};
use crate::style::Color;
use crate::text::Key;

/// Eine Zeichenoperation, in Malreihenfolge.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Rect {
        rect: Rect,
        fill: Option<Color>,
        stroke: Option<Color>,
        /// Eckenradius.
        radius: i32,
        /// Gestrichelter Rahmen.
        dashed: bool,
    },
    /// Einzeiliger Text; `x`/`y` ist die linke obere Ecke der Zeile.
    Text {
        x: i32,
        y: i32,
        text: String,
        color: Color,
        /// Schriftgröße in Pixeln.
        size: i32,
        bold: bool,
    },
    Line {
        from: Point,
        to: Point,
        color: Color,
    },
    /// Text in Festbreitenschrift (z. B. eine Zeile eines Terminals); mit
    /// `background` wird die Fläche hinter dem Text gefüllt.
    MonoText {
        x: i32,
        y: i32,
        text: String,
        color: Color,
        background: Option<Color>,
        /// Schriftgröße in Pixeln.
        size: i32,
        bold: bool,
        italic: bool,
        underline: bool,
    },
}

/// Gedrückte Zusatztasten.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    /// Befehlstaste (macOS) bzw. Super-/Windows-Taste.
    pub meta: bool,
}

/// Was mit der Maus geschah.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseAction {
    Pressed,
    Released,
    /// Bewegung (mit oder ohne gedrückte Taste).
    Moved,
    DoubleClicked,
    /// Mausrad nach oben bzw. unten (eine Raste).
    ScrollUp,
    ScrollDown,
}

/// Ein Mausereignis auf der Zeichenfläche, Koordinaten relativ zu ihr.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseEvent {
    pub action: MouseAction,
    pub x: i32,
    pub y: i32,
    /// 1 = links, 2 = Mitte, 3 = rechts; 0 bei Bewegung.
    pub button: u8,
    pub modifiers: Modifiers,
}

/// Ein Tastendruck auf der Zeichenfläche (sie hat dann den Fokus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
    /// Das Zeichen, wie getippt (mit Umschalt, AltGr …; z. B. `A`, `ä`,
    /// `@`); `None` bei Tasten ohne Zeichen wie Pfeilen.
    pub text: Option<char>,
}

/// Größe eines einzeiligen Texts in Pixeln (`monospace`: Festbreitenschrift,
/// wie [`Shape::MonoText`]) – z. B. für die Zellengröße eines Terminals.
pub fn measure_text(text: &str, size: i32, bold: bool, monospace: bool) -> crate::Size {
    use crate::backend::{Backend, Platform};
    Platform::measure_text(text, size, bold, monospace)
}
