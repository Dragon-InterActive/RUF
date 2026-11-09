//! Reine Rust-Geometrietypen – unabhängig vom Backend, damit Logik und
//! Designer ohne FFI damit rechnen können.

/// Wo ein zu langer einzeiliger Text gekürzt wird (`…`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Elide {
    /// Nicht kürzen (der Text darf über den Rand laufen).
    #[default]
    None,
    /// Am Anfang: `…/projekt/Cargo.toml`.
    Start,
    /// In der Mitte: `/home/…/Cargo.toml` (gut für Pfade).
    Middle,
    /// Am Ende: `Ein langer Te…`.
    End,
}

/// Ein Punkt in Pixeln, relativ zum Elternelement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Eine Größe in Pixeln.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

impl Size {
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

/// Ein Rechteck aus Position und Größe.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn from_parts(position: Point, size: Size) -> Self {
        Self::new(position.x, position.y, size.width, size.height)
    }

    pub const fn position(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub const fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    /// Liegt `point` innerhalb des Rechtecks (rechte/untere Kante exklusiv)?
    pub const fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x + self.width
            && point.y < self.y + self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_parts_roundtrip() {
        let r = Rect::from_parts(Point::new(10, 20), Size::new(30, 40));
        assert_eq!(r.position(), Point::new(10, 20));
        assert_eq!(r.size(), Size::new(30, 40));
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(0, 0, 10, 10);
        assert!(r.contains(Point::new(0, 0)));
        assert!(r.contains(Point::new(9, 9)));
        assert!(!r.contains(Point::new(10, 5)));
    }
}

/// An welchen Kanten des Elternelements ein Widget hängt (wie Delphis
/// `Anchors`). Ändert sich die Größe des Elternelements, behält das Widget
/// den Abstand zu den verankerten Kanten: links und rechts verankert wächst
/// es in der Breite mit, nur rechts verankert wandert es mit dem rechten Rand,
/// ohne linke und rechte Kante bleibt es mittig. Standard ist links oben.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Anchors {
    pub left: bool,
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
}

impl Default for Anchors {
    fn default() -> Self {
        Self::TOP_LEFT
    }
}

impl Anchors {
    /// Links oben (Standard): Position und Größe bleiben.
    pub const TOP_LEFT: Self = Self::new(true, true, false, false);
    /// Alle Kanten: wächst in Breite und Höhe mit.
    pub const ALL: Self = Self::new(true, true, true, true);
    /// Links, oben und rechts: wächst in der Breite mit.
    pub const TOP_STRETCH: Self = Self::new(true, true, true, false);
    /// Links, unten und rechts: wächst in der Breite mit, bleibt unten.
    pub const BOTTOM_STRETCH: Self = Self::new(true, false, true, true);
    /// Rechts oben: wandert mit dem rechten Rand.
    pub const TOP_RIGHT: Self = Self::new(false, true, true, false);
    /// Rechts unten: wandert mit der rechten unteren Ecke.
    pub const BOTTOM_RIGHT: Self = Self::new(false, false, true, true);
    /// Links unten: wandert mit dem unteren Rand.
    pub const BOTTOM_LEFT: Self = Self::new(true, false, false, true);

    pub const fn new(left: bool, top: bool, right: bool, bottom: bool) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Lage in einem Elternelement der Größe `parent`, wenn `rect` für die
    /// Größe `base` entworfen wurde.
    pub fn apply(self, rect: Rect, base: Size, parent: Size) -> Rect {
        let axis = |start: bool, end: bool, pos: i32, len: i32, delta: i32| match (start, end) {
            (true, true) => (pos, (len + delta).max(0)),
            (false, true) => (pos + delta, len),
            (false, false) => (pos + delta / 2, len),
            (true, false) => (pos, len),
        };
        let (x, width) = axis(
            self.left,
            self.right,
            rect.x,
            rect.width,
            parent.width - base.width,
        );
        let (y, height) = axis(
            self.top,
            self.bottom,
            rect.y,
            rect.height,
            parent.height - base.height,
        );
        Rect::new(x, y, width, height)
    }
}

#[cfg(test)]
mod anchor_tests {
    use super::*;

    #[test]
    fn anchors_follow_the_parent() {
        let r = Rect::new(10, 20, 100, 30);
        let base = Size::new(400, 300);
        let parent = Size::new(500, 360);
        assert_eq!(Anchors::TOP_LEFT.apply(r, base, parent), r);
        assert_eq!(
            Anchors::ALL.apply(r, base, parent),
            Rect::new(10, 20, 200, 90)
        );
        assert_eq!(
            Anchors::BOTTOM_RIGHT.apply(r, base, parent),
            Rect::new(110, 80, 100, 30)
        );
        assert_eq!(
            Anchors::new(false, false, false, false).apply(r, base, parent),
            Rect::new(60, 50, 100, 30)
        );
        // Kleiner als entworfen: nie negative Größe.
        assert_eq!(Anchors::ALL.apply(r, base, Size::new(100, 100)).width, 0);
    }
}
