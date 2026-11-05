//! Zeichenfläche: Formen, die das Programm vorgibt, und Maus-/Tastatur-
//! ereignisse, die es zurückbekommt.
//!
//! Die Zeichenfläche ([`crate::WidgetKind::Canvas`]) malt eine Liste von
//! [`Shape`]s – mehr nicht. Was gezeichnet wird, entscheidet das Programm
//! (z. B. der Formular-Designer von Freya TDE aus seiner Zeichenliste).

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
        italic: bool,
        /// Schriftfamilie (`None` = die des Widgets).
        family: Option<String>,
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
    /// Ein Bild in `rect`, eingepasst nach `fit`.
    Image {
        rect: Rect,
        image: Image,
        fit: ImageFit,
    },
}

/// Wie ein Bild in sein Rechteck kommt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ImageFit {
    /// Ganz sichtbar, Seitenverhältnis bleibt, mittig.
    #[default]
    Contain,
    /// Füllt das Rechteck, Seitenverhältnis bleibt, Überstand abgeschnitten.
    Cover,
    /// Füllt das Rechteck, verzerrt.
    Stretch,
    /// In Originalgröße mittig, Überstand abgeschnitten.
    Center,
}

/// Ein Bild für [`Shape::Image`]: PNG oder JPEG (SVG nur, wenn das System
/// einen Lader dafür hat). Klone teilen die Daten; das Backend dekodiert
/// jedes Bild nur einmal.
#[derive(Clone)]
pub struct Image {
    id: u64,
    bytes: std::sync::Arc<[u8]>,
    display: Option<crate::Size>,
}

impl Image {
    /// Ein Bild aus Dateidaten, z. B. `include_bytes!("logo.png")`.
    pub fn from_bytes(bytes: impl Into<std::sync::Arc<[u8]>>) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            bytes: bytes.into(),
            display: None,
        }
    }

    /// Anzeigegröße auf Knöpfen in logischen Pixeln, z. B. ein 48-px-Bild
    /// als 24 × 24 (scharf auf HiDPI-Bildschirmen). Ohne Angabe: Pixelgröße.
    pub fn with_display_size(mut self, width: i32, height: i32) -> Self {
        self.display = Some(crate::Size::new(width, height));
        self
    }

    pub fn display_size(&self) -> Option<crate::Size> {
        self.display.or_else(|| self.size())
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Kennung für den Zwischenspeicher des Backends.
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// Größe in Pixeln (`None`, wenn sich das Bild nicht lesen lässt).
    pub fn size(&self) -> Option<crate::Size> {
        use crate::backend::{Backend, Platform};
        Platform::image_size(self)
    }
}

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image(#{}, {} Bytes)", self.id, self.bytes.len())
    }
}

impl PartialEq for Image {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id || self.bytes == other.bytes
    }
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

/// Was beim Ziehen und Ablegen (Drag and drop) über der Zeichenfläche geschah.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropAction {
    /// Etwas wird über die Fläche gezogen.
    Motion,
    /// Die Maus hat die Fläche wieder verlassen (ohne Ablegen).
    Leave,
    /// Abgelegt.
    Drop,
}

/// Ziehen und Ablegen über der Zeichenfläche; `text` ist der gezogene Text
/// (siehe [`crate::Widget::set_drag_text`]), bei `Motion` sobald bekannt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DropEvent {
    pub action: DropAction,
    pub x: i32,
    pub y: i32,
    pub text: Option<String>,
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
