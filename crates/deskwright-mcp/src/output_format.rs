use schemars::JsonSchema;
use serde::Deserialize;

/// How element results are returned to the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// A compact indented outline, one line per element (default; fastest to read).
    #[default]
    Text,
    /// Full JSON including on-screen bounds.
    Json,
}
