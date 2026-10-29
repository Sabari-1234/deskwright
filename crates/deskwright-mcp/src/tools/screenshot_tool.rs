use std::sync::Arc;

use async_trait::async_trait;
use deskwright_core::model::app::AppSelector;
use deskwright_core::model::element::ElementRef;
use deskwright_core::model::encoded_image::{CaptureOptions, CaptureResolution, ImageFormat};
use deskwright_core::model::window::WindowSelector;
use deskwright_core::services::screenshot_service::{ScreenshotService, ScreenshotTarget};
use deskwright_core::{DeskwrightError, DeskwrightResult};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::blocking::run_blocking;
use crate::tool::{ToolContext, ToolOutput};
use crate::typed_tool::TypedTool;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ResolutionArgument {
    /// One pixel per screen point (default): a quarter of the pixels on Retina, still readable.
    #[default]
    Points,
    /// Every physical pixel, for fine detail.
    Pixels,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FormatArgument {
    /// Lossless (default).
    #[default]
    Png,
    /// Several times smaller; enough for reading UI state.
    Jpeg,
}

#[derive(Deserialize, JsonSchema)]
pub struct ScreenshotArguments {
    /// Element ref to capture. Omit it (and app/title/index) for the whole main display.
    #[serde(rename = "ref")]
    element_ref: Option<String>,
    /// Capture one of this app's windows instead: its own pixels, even when covered.
    app: Option<String>,
    /// Window title to look for (contains, ignoring case), with `app`.
    title: Option<String>,
    /// Window position among the matches, 0 = frontmost (default), with `app`.
    index: Option<usize>,
    /// "points" (default) or "pixels".
    #[serde(default)]
    resolution: ResolutionArgument,
    /// "png" (default) or "jpeg".
    #[serde(default)]
    format: FormatArgument,
    /// JPEG quality 1–100 (default 80).
    quality: Option<u8>,
}

impl ScreenshotArguments {
    fn target(&self) -> DeskwrightResult<ScreenshotTarget> {
        let names_window = self.app.is_some() || self.title.is_some() || self.index.is_some();
        match (&self.element_ref, names_window) {
            (Some(_), true) => Err(DeskwrightError::InvalidInput(
                "give either ref or a window (app/title/index), not both".to_owned(),
            )),
            (Some(raw_ref), false) => Ok(ScreenshotTarget::Element(ElementRef::parse(raw_ref)?)),
            (None, true) => Ok(ScreenshotTarget::Window {
                app: AppSelector::from_optional_name(self.app.as_deref()),
                window: WindowSelector {
                    title: self.title.clone(),
                    index: self.index,
                },
            }),
            (None, false) => Ok(ScreenshotTarget::MainDisplay),
        }
    }

    fn options(&self) -> DeskwrightResult<CaptureOptions> {
        let format = match self.format {
            FormatArgument::Png if self.quality.is_some() => {
                return Err(DeskwrightError::InvalidInput(
                    "quality only applies to format \"jpeg\"".to_owned(),
                ));
            }
            FormatArgument::Png => ImageFormat::Png,
            FormatArgument::Jpeg => ImageFormat::jpeg(self.quality)?,
        };
        let resolution = match self.resolution {
            ResolutionArgument::Points => CaptureResolution::Points,
            ResolutionArgument::Pixels => CaptureResolution::Pixels,
        };
        Ok(CaptureOptions { resolution, format })
    }
}

/// Captures an element, a window or the main display as an image.
pub struct ScreenshotTool {
    screenshots: Arc<ScreenshotService>,
}

impl ScreenshotTool {
    pub fn new(screenshots: Arc<ScreenshotService>) -> Self {
        Self { screenshots }
    }
}

#[async_trait]
impl TypedTool for ScreenshotTool {
    type Arguments = ScreenshotArguments;

    const NAME: &'static str = "desk_screenshot";
    const DESCRIPTION: &'static str = "Capture an image of one element (`ref`), one window \
        (`app`, optional `title`/`index`; its own pixels even when other windows cover it) or \
        the main display. Defaults to point resolution PNG; `format: \"jpeg\"` is several \
        times smaller, and `resolution: \"pixels\"` keeps full Retina detail. Prefer \
        desk_snapshot for understanding UI; use this for visual content. Needs Screen \
        Recording permission.";
    const IS_READ_ONLY: bool = true;
    const IS_DESTRUCTIVE: bool = false;

    async fn run(&self, arguments: ScreenshotArguments, _context: &ToolContext<'_>) -> ToolOutput {
        let parsed = arguments
            .target()
            .and_then(|target| Ok((target, arguments.options()?)));
        let (target, options) = match parsed {
            Ok(parsed) => parsed,
            Err(invalid_arguments) => return invalid_arguments.into(),
        };
        let screenshots = self.screenshots.clone();
        run_blocking(move || screenshots.capture(&target, &options))
            .await
            .map(|image| ToolOutput::image(&image))
            .into()
    }
}
