use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::DeskwrightResult;
use deskwright_core::model::app::AppSelector;
use deskwright_core::model::element_target::ElementTarget;
use deskwright_core::services::element_locator_service::ElementLocatorService;
use deskwright_core::services::interaction::text_entry_service::TextEntryService;
use deskwright_core::services::keyboard_service::KeyboardService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::element_target_arguments::ElementTargetArguments;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct TypeTextArguments {
    #[serde(flatten)]
    target: ElementTargetArguments,
    /// The text to enter.
    text: String,
}

/// Enters text into an element (value first, then keyboard typing), or types into the focused
/// element of the frontmost app when no element is given.
pub struct TypeTextTool {
    locator: Arc<ElementLocatorService>,
    text_entry: Arc<TextEntryService>,
    keyboard: Arc<KeyboardService>,
}

impl TypeTextTool {
    pub fn new(
        locator: Arc<ElementLocatorService>,
        text_entry: Arc<TextEntryService>,
        keyboard: Arc<KeyboardService>,
    ) -> Self {
        Self {
            locator,
            text_entry,
            keyboard,
        }
    }
}

#[async_trait]
impl TypedTool for TypeTextTool {
    type Arguments = TypeTextArguments;

    const NAME: &'static str = "desk_type";
    const DESCRIPTION: &'static str = "Enter text into an element given by `ref` or by a \
        locator (role/title/value in `app`, `nth` match), which waits for the element: replaces \
        an editable field's content, or focuses the element and types at the cursor. With no \
        element: types into whatever has keyboard focus in the frontmost app (refused unless \
        `app`, when given, is frontmost).";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = true;

    async fn run(&self, arguments: TypeTextArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let target = match arguments.target.to_target() {
            Ok(target) => target,
            Err(invalid_target) => return invalid_target.into(),
        };
        let result = match target {
            Some(target) => self.enter_into_element(target, arguments.text).await,
            None => {
                let app = arguments.target.app_selector();
                self.type_into_focus(arguments.text, app).await
            }
        };
        // Confirmations deliberately omit the text, which may be a password.
        result.into()
    }
}

impl TypeTextTool {
    async fn enter_into_element(
        &self,
        target: ElementTarget,
        text: String,
    ) -> DeskwrightResult<ToolOutput> {
        let locator = self.locator.clone();
        let text_entry = self.text_entry.clone();
        let (element_ref, outcome) = run_blocking(move || {
            let element_ref = locator.resolve(&target)?;
            Ok((element_ref, text_entry.enter_text(&element_ref, &text)?))
        })
        .await?;
        Ok(ToolOutput::text(format!(
            "Entered text into {element_ref} using {}.",
            outcome.strategy_name
        )))
    }

    async fn type_into_focus(
        &self,
        text: String,
        app: AppSelector,
    ) -> DeskwrightResult<ToolOutput> {
        let keyboard = self.keyboard.clone();
        let app = run_blocking(move || keyboard.type_text(&text, &app)).await?;
        Ok(ToolOutput::text(format!(
            "Typed text into the focused element of {}.",
            app.name
        )))
    }
}
