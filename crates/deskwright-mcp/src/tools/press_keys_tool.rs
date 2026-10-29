use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::model::app::AppSelector;
use deskwright_core::model::key_chord::KeyChord;
use deskwright_core::services::keyboard_service::KeyboardService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct PressKeysArguments {
    /// One key chord: a key name or character, optionally after modifiers joined with "+",
    /// e.g. "Enter", "Escape", "Tab", "Cmd+L", "Cmd+Shift+G". Modifiers: Cmd, Ctrl, Option,
    /// Shift. Keys: Enter, Escape, Tab, Space, Backspace, Delete, Up, Down, Left, Right,
    /// Home, End, PageUp, PageDown, or any single character.
    keys: String,
    /// The app that must be frontmost to receive the keys. Omit to send to the frontmost app.
    app: Option<String>,
}

/// Presses a key or keyboard shortcut in the frontmost app.
pub struct PressKeysTool {
    keyboard: Arc<KeyboardService>,
}

impl PressKeysTool {
    pub fn new(keyboard: Arc<KeyboardService>) -> Self {
        Self { keyboard }
    }
}

#[async_trait]
impl TypedTool for PressKeysTool {
    type Arguments = PressKeysArguments;

    const NAME: &'static str = "desk_press_keys";
    const DESCRIPTION: &'static str = "Press a key or keyboard shortcut, e.g. Enter to submit a \
        search, Escape to close a menu, or Cmd+S. Keys go to the frontmost app; when `app` is \
        given, the call is refused unless that app is frontmost (use desk_launch_app first). \
        Shortcuts can act immediately (Cmd+Q quits), so press only what the user asked for.";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = true;

    async fn run(&self, arguments: PressKeysArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let chord = match KeyChord::parse(&arguments.keys) {
            Ok(chord) => chord,
            Err(invalid_chord) => return invalid_chord.into(),
        };
        let target = AppSelector::from_optional_name(arguments.app.as_deref());
        let keyboard = self.keyboard.clone();
        let pressed_chord = chord.clone();
        run_blocking(move || keyboard.press(&pressed_chord, &target))
            .await
            .map(|app| ToolOutput::text(format!("Pressed {chord} in {}.", app.name)))
            .into()
    }
}
