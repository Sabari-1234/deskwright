use deskwright::ToolSpec;

/// One line per tool: its name and the first sentence of its description.
pub fn render_tool_list(tools: &[ToolSpec]) -> String {
    tools
        .iter()
        .map(|tool| format!("{:<24} {}", tool.name, first_sentence(tool.description)))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn first_sentence(description: &str) -> &str {
    description
        .split_once(". ")
        .map_or(description, |(sentence, _)| sentence)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(name: &'static str, description: &'static str) -> ToolSpec {
        ToolSpec {
            name,
            description,
            input_schema: serde_json::Map::new(),
            is_read_only: true,
            is_destructive: false,
        }
    }

    #[test]
    fn lists_names_with_the_first_sentence() {
        let tools = [
            spec("desk_read", "Read an element. Returns its state."),
            spec("desk_list_apps", "List running apps"),
        ];

        assert_eq!(
            render_tool_list(&tools),
            "desk_read                Read an element\n\
             desk_list_apps           List running apps\n"
        );
    }
}
