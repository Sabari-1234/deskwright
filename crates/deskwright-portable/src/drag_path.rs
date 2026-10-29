use deskwright_core::model::geometry::Point;

/// The points a drag moves through after leaving `from`, evenly spaced and ending exactly
/// at `to`. Apps only see a drag when the pointer moves in several steps while the button is
/// held, like a person's hand; one jump is often treated as a click.
pub fn drag_path(from: Point, to: Point, steps: usize) -> Vec<Point> {
    let steps = steps.max(1);
    (1..=steps)
        .map(|step| {
            let progress = step as f64 / steps as f64;
            Point {
                x: from.x + (to.x - from.x) * progress,
                y: from.y + (to.y - from.y) * progress,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_in_even_steps_and_ends_at_the_target() {
        let path = drag_path(Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 50.0 }, 4);

        assert_eq!(
            path,
            [
                Point { x: 25.0, y: 12.5 },
                Point { x: 50.0, y: 25.0 },
                Point { x: 75.0, y: 37.5 },
                Point { x: 100.0, y: 50.0 },
            ]
        );
    }

    #[test]
    fn always_takes_at_least_one_step() {
        let target = Point { x: 3.0, y: 4.0 };

        assert_eq!(drag_path(Point { x: 0.0, y: 0.0 }, target, 0), [target]);
    }
}
