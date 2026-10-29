//! Converts this crate's tool types to and from `rmcp` protocol types.

use std::sync::Arc;

use rmcp::model::{CallToolResult, ContentBlock, Tool as McpTool, ToolAnnotations};

use crate::tool::{OutputBlock, ToolOutput, ToolSpec};

pub fn to_mcp_tool(spec: ToolSpec) -> McpTool {
    let annotations = ToolAnnotations::new()
        .read_only(spec.is_read_only)
        .destructive(spec.is_destructive);
    McpTool::new(spec.name, spec.description, Arc::new(spec.input_schema))
        .with_annotations(annotations)
}

pub fn to_call_tool_result(output: ToolOutput) -> CallToolResult {
    let content = output.blocks.into_iter().map(to_content_block).collect();
    if output.is_error {
        CallToolResult::error(content)
    } else {
        CallToolResult::success(content)
    }
}

fn to_content_block(block: OutputBlock) -> ContentBlock {
    match block {
        OutputBlock::Text(text) => ContentBlock::text(text),
        OutputBlock::Image {
            base64_data,
            mime_type,
        } => ContentBlock::image(base64_data, mime_type),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::JsonObject;

    fn spec(is_read_only: bool, is_destructive: bool) -> ToolSpec {
        ToolSpec {
            name: "desk_example",
            description: "Example tool.",
            input_schema: JsonObject::from_iter([("type".to_owned(), "object".into())]),
            is_read_only,
            is_destructive,
        }
    }

    #[test]
    fn carries_name_description_schema_and_hints_into_the_mcp_tool() {
        let mcp_tool = to_mcp_tool(spec(true, false));

        assert_eq!(mcp_tool.name, "desk_example");
        assert_eq!(mcp_tool.description.as_deref(), Some("Example tool."));
        assert_eq!(mcp_tool.input_schema.get("type"), Some(&"object".into()));
        let annotations = mcp_tool.annotations.expect("annotations are set");
        assert_eq!(annotations.read_only_hint, Some(true));
        assert_eq!(annotations.destructive_hint, Some(false));
    }

    #[test]
    fn maps_success_output_blocks_in_order() {
        let output = ToolOutput {
            blocks: vec![
                OutputBlock::Text("caption".to_owned()),
                OutputBlock::Image {
                    base64_data: "/9j/4A==".to_owned(),
                    mime_type: "image/jpeg",
                },
            ],
            is_error: false,
        };

        let result = to_call_tool_result(output);

        assert_eq!(result.is_error, Some(false));
        assert!(matches!(&result.content[0], ContentBlock::Text(text) if text.text == "caption"));
        assert!(matches!(
            &result.content[1],
            ContentBlock::Image(image) if image.mime_type == "image/jpeg" && image.data == "/9j/4A=="
        ));
    }

    #[test]
    fn marks_error_output_as_an_error_result() {
        let result = to_call_tool_result(ToolOutput::error("nope"));

        assert_eq!(result.is_error, Some(true));
    }
}
