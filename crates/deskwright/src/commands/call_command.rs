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
            OutputBlock::Image {
                base64_data,
                mime_type,
            } => format!(
                "[{mime_type} image, {} base64 characters]",
                base64_data.len()
            ),
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn save_images(output: &ToolOutput, tool: &str, image_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(image_dir)?;
    let mut saved = Vec::new();
    let images = output.blocks.iter().filter_map(|block| match block {
        OutputBlock::Image {
            base64_data,
            mime_type,
        } => Some((base64_data, file_extension(mime_type))),
        OutputBlock::Text(_) => None,
    });
    for (index, (base64_data, extension)) in images.enumerate() {
        let path = image_dir.join(format!("{tool}-{index}.{extension}"));
        std::fs::write(&path, BASE64_STANDARD.decode(base64_data)?)?;
        saved.push(path);
    }
    Ok(saved)
}

/// The file extension for an image MIME type, defaulting to the subtype ("image/png" → "png").
fn file_extension(mime_type: &str) -> &str {
    match mime_type {
        "image/jpeg" => "jpg",
        other => other.rsplit('/').next().unwrap_or("img"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output_with_image(mime_type: &'static str) -> ToolOutput {
        ToolOutput {
            blocks: vec![
                OutputBlock::Text("Captured.".to_owned()),
                OutputBlock::Image {
                    base64_data: BASE64_STANDARD.encode(b"png bytes"),
                    mime_type,
                },
            ],
            is_error: false,
        }
    }

    #[test]
    fn renders_text_and_summarizes_images() {
        assert_eq!(
            render_output(&output_with_image("image/png")),
            "Captured.\n[image/png image, 12 base64 characters]\n"
        );
    }

    #[test]
    fn saves_each_image_with_the_extension_of_its_format() {
        let directory =
            std::env::temp_dir().join(format!("deskwright-call-{}", std::process::id()));

        let png = save_images(
            &output_with_image("image/png"),
            "desk_screenshot",
            &directory,
        )
        .unwrap();
        let jpeg = save_images(
            &output_with_image("image/jpeg"),
            "desk_screenshot",
            &directory,
        )
        .unwrap();

        assert_eq!(png, [directory.join("desk_screenshot-0.png")]);
        assert_eq!(jpeg, [directory.join("desk_screenshot-0.jpg")]);
        assert_eq!(std::fs::read(&png[0]).unwrap(), b"png bytes");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
