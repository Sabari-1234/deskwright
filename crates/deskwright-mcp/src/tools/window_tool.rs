use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::model::app::AppSelector;
use deskwright_core::model::window::{WindowCommand, WindowSelector, WindowSummary};
use deskwright_core::services::window_service::WindowService;
use deskwright_core::{DeskwrightError, DeskwrightResult};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WindowAction {
    List,
    Minimize,
    Restore,
    Close,
    Raise,
    Move,
    Resize,
}

#[derive(Deserialize, JsonSchema)]
pub struct WindowArguments {
    /// What to do. "list" returns the app's windows; the others act on one window.
    action: WindowAction,
    /// App name or bundle ID. Omit for the frontmost app.
    app: Option<String>,
    /// Pick the window whose title contains this text (ignoring case).
    title: Option<String>,
    /// Pick the window at this position among the matches, 0 = frontmost (default).
    index: Option<usize>,
    /// "move": the new left edge, in screen points.
    x: Option<f64>,
    /// "move": the new top edge, in screen points.
    y: Option<f64>,
    /// "resize": the new width, in points.
    width: Option<f64>,
    /// "resize": the new height, in points.
    height: Option<f64>,
}

impl WindowArguments {
    /// `None` for "list".
    fn command(&self) -> DeskwrightResult<Option<WindowCommand>> {
        let command = match self.action {
            WindowAction::List => return Ok(None),
            WindowAction::Minimize => WindowCommand::Minimize,
            WindowAction::Restore => WindowCommand::Restore,
            WindowAction::Close => WindowCommand::Close,
            WindowAction::Raise => WindowCommand::Raise,
            WindowAction::Move => WindowCommand::move_to(
                required(self.x, "x", "move")?,
                required(self.y, "y", "move")?,
            )?,
            WindowAction::Resize => WindowCommand::resize(
                required(self.width, "width", "resize")?,
                required(self.height, "height", "resize")?,
            )?,
        };
        Ok(Some(command))
    }

    fn window_selector(&self) -> WindowSelector {
        WindowSelector {
            title: self.title.clone(),
            index: self.index,
        }
    }
}

fn required(value: Option<f64>, name: &str, action: &str) -> DeskwrightResult<f64> {
    value.ok_or_else(|| DeskwrightError::InvalidInput(format!("\"{action}\" needs {name}")))
}

/// Lists, minimizes, restores, closes, raises, moves or resizes an app's windows.
pub struct WindowTool {
    windows: Arc<WindowService>,
}

impl WindowTool {
    pub fn new(windows: Arc<WindowService>) -> Self {
        Self { windows }
    }
}

#[async_trait]
impl TypedTool for WindowTool {
    type Arguments = WindowArguments;

    const NAME: &'static str = "desk_window";
    const DESCRIPTION: &'static str = "Manage an app's windows. `action`: \"list\" (titles, \
        bounds, minimized), or \"minimize\", \"restore\", \"close\" (the app may ask to save), \
        \"raise\" (bring to the front, with its app), \"move\" (`x`, `y`) or \"resize\" \
        (`width`, `height`) one window, picked by `title` (contains) and/or `index` (0 = \
        frontmost, the default).";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = true;

    async fn run(&self, arguments: WindowArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let command = match arguments.command() {
            Ok(command) => command,
            Err(invalid_arguments) => return invalid_arguments.into(),
        };
        let app = AppSelector::from_optional_name(arguments.app.as_deref());
        let selector = arguments.window_selector();
        let windows = self.windows.clone();
        run_blocking(move || match command {
            None => {
                let (app, summaries) = windows.list(&app)?;
                Ok(ToolOutput::text(render_window_list(&app.name, &summaries)))
            }
            Some(command) => {
                let window = windows.apply(&app, &selector, command)?;
                Ok(ToolOutput::text(format!(
                    "{} {}.",
                    capitalize(command.past_tense()),
                    describe(&window)
                )))
            }
        })
        .await
        .into()
    }
}

fn render_window_list(app_name: &str, windows: &[WindowSummary]) -> String {
    if windows.is_empty() {
        return format!("{app_name} has no open windows.");
    }
    windows.iter().map(describe).collect::<Vec<_>>().join("\n")
}

/// One line per window: `[0] "Report.pages" at 10,20 size 800x600 (minimized)`.
fn describe(window: &WindowSummary) -> String {
    let mut line = format!("[{}]", window.index);
    match &window.title {
        Some(title) => line.push_str(&format!(" \"{title}\"")),
        None => line.push_str(" (untitled)"),
    }
    if let Some(bounds) = window.bounds {
        line.push_str(&format!(
            " at {},{} size {}x{}",
            bounds.x, bounds.y, bounds.width, bounds.height
        ));
    }
    if window.is_minimized {
        line.push_str(" (minimized)");
    }
    line
}

fn capitalize(text: &str) -> String {
    let mut characters = text.chars();
    characters
        .next()
        .map(|first| first.to_uppercase().chain(characters).collect())
        .unwrap_or_default()
}
