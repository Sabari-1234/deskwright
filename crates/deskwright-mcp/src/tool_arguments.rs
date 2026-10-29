use schemars::JsonSchema;
use serde::de::DeserializeOwned;

use crate::tool::{JsonObject, ToolOutput};

/// Deserializes a tool's arguments, turning failures into a readable error result.
pub fn parse_arguments<Arguments: DeserializeOwned>(
    arguments: JsonObject,
) -> Result<Arguments, ToolOutput> {
    serde_json::from_value(serde_json::Value::Object(arguments))
        .map_err(|error| ToolOutput::error(format!("invalid arguments: {error}")))
}

/// The JSON Schema object describing `Arguments`, derived from its type and doc comments.
pub fn input_schema_for<Arguments: JsonSchema>() -> JsonObject {
    let schema = schemars::schema_for!(Arguments);
    match serde_json::to_value(schema) {
        Ok(serde_json::Value::Object(schema_object)) => schema_object,
        // A derived schema always serializes to an object; fall back to "accepts any object".
        _ => JsonObject::from_iter([("type".to_owned(), "object".into())]),
    }
}
