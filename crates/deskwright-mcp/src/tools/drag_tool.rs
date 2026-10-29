use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::services::interaction::pointer_gesture_service::PointerGestureService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::element_target_arguments::ElementTargetArguments;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct DragArguments {
    /// The element to drag: a `ref` or a locator, like any element argument.
    from: ElementTargetArguments,
    /// Where to drop it: a `ref` or a locator.
    to: ElementTargetArguments,
}

/// Drags one element onto another with the mouse.
pub struct DragTool {
    gestures: Arc<PointerGestureService>,
}

impl DragTool {
    pub fn new(gestures: Arc<PointerGestureService>) -> Self {
        Self { gestures }
    }
}

#[async_trait]
impl TypedTool for DragTool {
    type Arguments = DragArguments;

    const NAME: &'static str = "desk_drag";
    const DESCRIPTION: &'static str = "Drag an element onto another with the mouse, e.g. a file \
        into a folder or a card into a column. `from` and `to` each take a `ref` or a locator \
        (role/title/value/… in `app`). The drop target may be in another app.";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = true;

    async fn run(&self, arguments: DragArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let targets = arguments
            .from
            .to_required_target()
            .and_then(|from| Ok((from, arguments.to.to_required_target()?)));
        let (from, to) = match targets {
            Ok(targets) => targets,
            Err(invalid_target) => return invalid_target.into(),
        };
        let gestures = self.gestures.clone();
        run_blocking(move || gestures.drag(&from, &to))
            .await
            .map(|()| ToolOutput::text("Dragged the element onto the drop target."))
            .into()
    }
}
