use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use deskwright::{Deskwright, OutputBlock, ToolOutput};

/// Runs one tool and returns what to print, saving images to `image_dir` when given.
pub async fn call(
    deskwright: &Deskwright,
    tool: &str,
    raw_arguments: &str,
    image_dir: Option<&Path>,
) -> anyhow::Result<ToolOutput> {
    let arguments: serde_json::Value = serde_json::from_str(raw_arguments)
        .map_err(|error| anyhow::anyhow!("the arguments are not valid JSON: {error}"))?;
    let output = deskwright.call(tool, arguments).await;
    if let Some(image_dir) = image_dir {
        save_images(&output, tool, image_dir)?;
    }
    Ok(output)
}

/// The output as printable text; images are summarized, since they can't be printed.
pub fn render_output(output: &ToolOutput) -> String {
    output
        .blocks
        .iter()
        .map(|block| match block {
            OutputBlock::Text(text) => text.clone(),
            OutputBlock::PngImage { base64_data } => {
                format!("[PNG image, {} base64 characters]", base64_data.len())
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn save_images(output: &ToolOutput, tool: &str, image_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(image_dir)?;
    let mut saved = Vec::new();
    let images = output.blocks.iter().filter_map(|block| match block {
        OutputBlock::PngImage { base64_data } => Some(base64_data),
        OutputBlock::Text(_) => None,
    });
    for (index, base64_data) in images.enumerate() {
        let path = image_dir.join(format!("{tool}-{index}.png"));
        std::fs::write(&path, BASE64_STANDARD.decode(base64_data)?)?;
        saved.push(path);
    }
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output_with_image() -> ToolOutput {
        ToolOutput {
            blocks: vec![
                OutputBlock::Text("Captured.".to_owned()),
                OutputBlock::PngImage {
                    base64_data: BASE64_STANDARD.encode(b"png bytes"),
                },
            ],
            is_error: false,
        }
    }

    #[test]
    fn renders_text_and_summarizes_images() {
        assert_eq!(
            render_output(&output_with_image()),
            "Captured.\n[PNG image, 12 base64 characters]\n"
        );
    }

    #[test]
    fn saves_each_image_as_a_png_file() {
        let directory =
            std::env::temp_dir().join(format!("deskwright-call-{}", std::process::id()));

        let saved = save_images(&output_with_image(), "desk_screenshot", &directory).unwrap();

        assert_eq!(saved, [directory.join("desk_screenshot-0.png")]);
        assert_eq!(std::fs::read(&saved[0]).unwrap(), b"png bytes");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
