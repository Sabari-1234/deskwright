use serde::Serialize;

/// A point in global screen coordinates (points, origin at the top-left of the main display).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// A width and height in points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

/// A rectangle in global screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Bounds {
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}
