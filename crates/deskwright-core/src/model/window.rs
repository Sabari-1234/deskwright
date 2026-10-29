use serde::Serialize;

use crate::error::{DeskwrightError, DeskwrightResult};
use crate::model::geometry::{Bounds, Point, Size};

/// One of an app's windows, as listed by `desk_window`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSummary {
    /// Position in the app's window list, frontmost first.
    pub index: usize,
    pub title: Option<String>,
    pub bounds: Option<Bounds>,
    pub is_minimized: bool,
}

/// Which of an app's windows to act on: the `index`-th window whose title contains `title`
/// (ignoring case), or the frontmost window when neither is given.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowSelector {
    pub title: Option<String>,
    pub index: Option<usize>,
}

impl WindowSelector {
    pub fn matches_title(&self, window_title: Option<&str>) -> bool {
        match &self.title {
            None => true,
            Some(wanted) => window_title
                .is_some_and(|title| title.to_lowercase().contains(&wanted.to_lowercase())),
        }
    }
}

impl std::fmt::Display for WindowSelector {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.title, self.index) {
            (None, None) => write!(formatter, "open window"),
            (None, Some(index)) => write!(formatter, "window at index {index}"),
            (Some(title), None) => write!(formatter, "window titled \"{title}\""),
            (Some(title), Some(index)) => write!(
                formatter,
                "window at index {index} among those titled \"{title}\""
            ),
        }
    }
}

/// What to do to a window. Each variant is one window-management action.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowCommand {
    Minimize,
    Restore,
    Close,
    /// Bring the window and its app to the front.
    Raise,
    MoveTo(Point),
    Resize(Size),
}

impl WindowCommand {
    /// Rejects sizes that no window can have.
    pub fn resize(width: f64, height: f64) -> DeskwrightResult<Self> {
        if width > 0.0 && height > 0.0 && width.is_finite() && height.is_finite() {
            Ok(Self::Resize(Size { width, height }))
        } else {
            Err(DeskwrightError::InvalidInput(
                "width and height must be positive numbers".to_owned(),
            ))
        }
    }

    pub fn move_to(x: f64, y: f64) -> DeskwrightResult<Self> {
        if x.is_finite() && y.is_finite() {
            Ok(Self::MoveTo(Point { x, y }))
        } else {
            Err(DeskwrightError::InvalidInput(
                "x and y must be numbers".to_owned(),
            ))
        }
    }

    /// Past-tense description for confirmations, like "minimized".
    pub fn past_tense(&self) -> &'static str {
        match self {
            Self::Minimize => "minimized",
            Self::Restore => "restored",
            Self::Close => "closed",
            Self::Raise => "brought to the front",
            Self::MoveTo(_) => "moved",
            Self::Resize(_) => "resized",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_titles_by_case_insensitive_substring() {
        let selector = WindowSelector {
            title: Some("report".to_owned()),
            index: None,
        };

        assert!(selector.matches_title(Some("Q3 Report.pages")));
        assert!(!selector.matches_title(Some("Notes")));
        assert!(!selector.matches_title(None));
        assert!(WindowSelector::default().matches_title(None));
    }

    #[test]
    fn describes_the_selection() {
        let both = WindowSelector {
            title: Some("Report".to_owned()),
            index: Some(1),
        };

        assert_eq!(WindowSelector::default().to_string(), "open window");
        assert_eq!(
            WindowSelector {
                index: Some(2),
                ..WindowSelector::default()
            }
            .to_string(),
            "window at index 2"
        );
        assert_eq!(
            WindowSelector {
                title: Some("Report".to_owned()),
                index: None
            }
            .to_string(),
            "window titled \"Report\""
        );
        assert_eq!(
            both.to_string(),
            "window at index 1 among those titled \"Report\""
        );
    }

    #[test]
    fn validates_geometry() {
        assert!(WindowCommand::resize(800.0, 600.0).is_ok());
        assert!(WindowCommand::resize(0.0, 600.0).is_err());
        assert!(WindowCommand::resize(f64::NAN, 600.0).is_err());
        assert!(WindowCommand::move_to(-100.0, 20.0).is_ok());
        assert!(WindowCommand::move_to(f64::INFINITY, 20.0).is_err());
    }

    #[test]
    fn names_each_command_in_the_past_tense() {
        let names: Vec<_> = [
            WindowCommand::Minimize,
            WindowCommand::Restore,
            WindowCommand::Close,
            WindowCommand::Raise,
            WindowCommand::MoveTo(Point { x: 0.0, y: 0.0 }),
            WindowCommand::Resize(Size {
                width: 1.0,
                height: 1.0,
            }),
        ]
        .iter()
        .map(WindowCommand::past_tense)
        .collect();

        assert_eq!(
            names,
            [
                "minimized",
                "restored",
                "closed",
                "brought to the front",
                "moved",
                "resized"
            ]
        );
    }
}
