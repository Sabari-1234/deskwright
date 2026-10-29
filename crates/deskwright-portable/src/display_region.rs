use deskwright_core::model::geometry::Bounds;
use deskwright_core::{DeskwrightError, DeskwrightResult};

/// A capture rectangle relative to one display's top-left corner, in points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl DisplayRegion {
    /// The part of `region` that lies on `display`, relative to the display's origin.
    /// Both rectangles are in global screen coordinates.
    pub fn clip(region: Bounds, display: Bounds) -> DeskwrightResult<Self> {
        let left = (region.x - display.x).clamp(0.0, display.width);
        let top = (region.y - display.y).clamp(0.0, display.height);
        let right = (region.x + region.width - display.x).clamp(0.0, display.width);
        let bottom = (region.y + region.height - display.y).clamp(0.0, display.height);
        if right - left < 1.0 || bottom - top < 1.0 {
            return Err(DeskwrightError::operation_failed(
                "capture region",
                "the region is not on the display",
            ));
        }
        // Every value is clamped to the display's size, which itself came from a u32,
        // so these casts cannot overflow or go negative.
        Ok(Self {
            x: left.floor() as u32,
            y: top.floor() as u32,
            width: (right - left).floor() as u32,
            height: (bottom - top).floor() as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN_DISPLAY: Bounds = Bounds {
        x: 0.0,
        y: 0.0,
        width: 1440.0,
        height: 900.0,
    };

    fn bounds(x: f64, y: f64, width: f64, height: f64) -> Bounds {
        Bounds {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn keeps_a_region_that_is_fully_on_the_display() {
        let clipped = DisplayRegion::clip(bounds(10.0, 20.0, 100.0, 50.0), MAIN_DISPLAY).unwrap();

        assert_eq!(
            clipped,
            DisplayRegion {
                x: 10,
                y: 20,
                width: 100,
                height: 50
            }
        );
    }

    #[test]
    fn trims_the_parts_that_hang_off_the_display() {
        let clipped = DisplayRegion::clip(bounds(-30.0, 880.0, 100.0, 50.0), MAIN_DISPLAY).unwrap();

        assert_eq!(
            clipped,
            DisplayRegion {
                x: 0,
                y: 880,
                width: 70,
                height: 20
            }
        );
    }

    #[test]
    fn converts_to_coordinates_relative_to_a_secondary_display() {
        let left_display = bounds(-1920.0, 0.0, 1920.0, 1080.0);

        let clipped =
            DisplayRegion::clip(bounds(-1900.0, 100.0, 200.0, 100.0), left_display).unwrap();

        assert_eq!(clipped.x, 20);
        assert_eq!(clipped.y, 100);
    }

    #[test]
    fn rejects_a_region_that_is_not_on_the_display() {
        assert!(DisplayRegion::clip(bounds(5000.0, 5000.0, 10.0, 10.0), MAIN_DISPLAY).is_err());
        assert!(DisplayRegion::clip(bounds(10.0, 10.0, 0.5, 10.0), MAIN_DISPLAY).is_err());
    }
}
