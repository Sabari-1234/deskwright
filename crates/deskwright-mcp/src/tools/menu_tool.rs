use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::model::app::AppSelector;
use deskwright_core::model::menu_path::MenuPath;
use deskwright_core::services::menu_service::MenuService;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Deserialize, JsonSchema)]
pub struct MenuArguments {
    /// Menu path separated by ">", e.g. "File > Save As…" (titles are case-insensitive).
    path: String,
    /// App name or bundle ID. Omit for the frontmost app.
    app: Option<String>,
}

/// Chooses a menu-bar item by path.
pub struct MenuTool {
    menus: Arc<MenuService>,
}

impl MenuTool {
    pub fn new(menus: Arc<MenuService>) -> Self {
        Self { menus }
    }
}

#[async_trait]
impl TypedTool for MenuTool {
    type Arguments = MenuArguments;

    const NAME: &'static str = "desk_menu";
    const DESCRIPTION: &'static str = "Choose a menu-bar item by path, e.g. \
        \"File > Save As…\", without opening the menus visually.";
    const IS_READ_ONLY: bool = false;
    const IS_DESTRUCTIVE: bool = true;

    async fn run(&self, arguments: MenuArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let path = match MenuPath::parse(&arguments.path) {
            Ok(path) => path,
            Err(invalid_path) => return invalid_path.into(),
        };
        let selector = AppSelector::from_optional_name(arguments.app.as_deref());
        let menus = self.menus.clone();
        let chosen_path = path.clone();
        run_blocking(move || menus.choose(&selector, &chosen_path))
            .await
            .map(|()| ToolOutput::text(format!("Chose menu item '{path}'.")))
            .into()
    }
}
