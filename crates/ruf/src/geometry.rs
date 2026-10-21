//! Reine Rust-Geometrietypen – unabhängig von Qt, damit Logik und Designer
//! ohne FFI damit rechnen können.

use cxx_qt_lib::QRect;

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

impl From<Rect> for QRect {
    fn from(r: Rect) -> Self {
        QRect::new(r.x, r.y, r.width, r.height)
    }
}

impl From<&QRect> for Rect {
    fn from(r: &QRect) -> Self {
        Rect::new(r.x(), r.y(), r.width(), r.height())
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
