use deskwright_core::model::app::ProcessId;
use deskwright_core::model::geometry::Bounds;

/// Window frames from two APIs can differ by rounding; this much counts as the same frame.
const FRAME_TOLERANCE_POINTS: f64 = 2.0;

/// An on-screen window as the capture library lists it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowCandidate {
    pub process_id: ProcessId,
    pub bounds: Bounds,
}

/// The index of the candidate that is the window of `process_id` at `bounds`: the process
/// must match, and so must the frame, so another window of the same app is never captured.
pub fn find_window(
    candidates: &[WindowCandidate],
    process_id: ProcessId,
    bounds: Bounds,
) -> Option<usize> {
    candidates.iter().position(|candidate| {
        candidate.process_id == process_id && has_same_frame(candidate.bounds, bounds)
    })
}

fn has_same_frame(first: Bounds, second: Bounds) -> bool {
    [
        (first.x, second.x),
        (first.y, second.y),
        (first.width, second.width),
        (first.height, second.height),
    ]
    .iter()
    .all(|(a, b)| (a - b).abs() <= FRAME_TOLERANCE_POINTS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(x: f64, width: f64) -> Bounds {
        Bounds {
            x,
            y: 40.0,
            width,
            height: 300.0,
        }
    }

    fn candidate(process_id: i32, bounds: Bounds) -> WindowCandidate {
        WindowCandidate {
            process_id: ProcessId(process_id),
            bounds,
        }
    }

    #[test]
    fn matches_process_and_frame_within_rounding() {
        let candidates = [
            candidate(1, bounds(100.0, 400.0)),
            candidate(2, bounds(100.0, 400.0)),
            candidate(2, bounds(600.0, 400.0)),
        ];

        assert_eq!(
            find_window(&candidates, ProcessId(2), bounds(601.5, 399.0)),
            Some(2)
        );
    }

    #[test]
    fn finds_nothing_for_another_frame_or_process() {
        let candidates = [candidate(1, bounds(100.0, 400.0))];

        assert_eq!(
            find_window(&candidates, ProcessId(1), bounds(100.0, 500.0)),
            None
        );
        assert_eq!(
            find_window(&candidates, ProcessId(9), bounds(100.0, 400.0)),
            None
        );
    }
}
