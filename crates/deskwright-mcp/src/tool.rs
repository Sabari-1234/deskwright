use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use deskwright_core::DeskwrightError;
use deskwright_core::model::png_image::PngImage;
use serde::Serialize;

pub type JsonObject = serde_json::Map<String, serde_json::Value>;

/// A tool's public contract, as advertised to MCP clients.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: JsonObject,
    /// The tool only reads state.
    pub is_read_only: bool,
    /// The tool can cause changes that are hard to undo (sending messages, running scripts).
    pub is_destructive: bool,
}

/// One piece of a tool's result.
#[derive(Debug, Clone, PartialEq)]
pub enum OutputBlock {
    Text(String),
    PngImage { base64_data: String },
}

/// A tool's result. Failures are results too (`is_error`), so the client can show them.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolOutput {
    pub blocks: Vec<OutputBlock>,
    pub is_error: bool,
}

impl ToolOutput {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            blocks: vec![OutputBlock::Text(text.into())],
            is_error: false,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            blocks: vec![OutputBlock::Text(message.into())],
            is_error: true,
        }
    }

    /// Pretty-printed JSON, so both people and models can read it.
    pub fn json(value: &impl Serialize) -> Self {
        match serde_json::to_string_pretty(value) {
            Ok(json) => Self::text(json),
            Err(error) => Self::error(format!("could not encode the result as JSON: {error}")),
        }
    }

    pub fn png(image: &PngImage) -> Self {
        Self {
            blocks: vec![OutputBlock::PngImage {
                base64_data: BASE64_STANDARD.encode(image.as_bytes()),
            }],
            is_error: false,
        }
    }
}

impl From<DeskwrightError> for ToolOutput {
    fn from(error: DeskwrightError) -> Self {
        Self::error(error.to_string())
    }
}

impl<Success: Into<ToolOutput>> From<Result<Success, DeskwrightError>> for ToolOutput {
    fn from(result: Result<Success, DeskwrightError>) -> Self {
        result.map_or_else(Self::from, Into::into)
    }
}

/// Calls tools by name; lets a tool (such as batch) invoke other tools.
#[async_trait]
pub trait ToolDispatcher: Send + Sync {
    async fn dispatch(&self, tool_name: &str, arguments: JsonObject) -> ToolOutput;
}

/// What a tool receives besides its arguments.
pub struct ToolContext<'call> {
    pub dispatcher: &'call dyn ToolDispatcher,
}

/// One MCP tool. Implementations parse their arguments, call a core service, and format
/// the result; they hold no business logic of their own.
#[async_trait]
pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;

    async fn call(&self, arguments: JsonObject, context: &ToolContext<'_>) -> ToolOutput;
}
