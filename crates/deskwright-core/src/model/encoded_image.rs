use crate::error::{DeskwrightError, DeskwrightResult};

/// Default JPEG quality: small files with text still readable.
pub const DEFAULT_JPEG_QUALITY: u8 = 80;

/// How a screenshot is encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageFormat {
    /// Lossless; best for small UI text and exact colors.
    #[default]
    Png,
    /// Lossy and several times smaller; `quality` is 1–100.
    Jpeg { quality: u8 },
}

impl ImageFormat {
    /// JPEG at `quality` (1–100), or the default quality.
    pub fn jpeg(quality: Option<u8>) -> DeskwrightResult<Self> {
        let quality = quality.unwrap_or(DEFAULT_JPEG_QUALITY);
        if (1..=100).contains(&quality) {
            Ok(Self::Jpeg { quality })
        } else {
            Err(DeskwrightError::InvalidInput(format!(
                "JPEG quality must be 1–100, got {quality}"
            )))
        }
    }

    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg { .. } => "image/jpeg",
        }
    }

    pub fn file_extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg { .. } => "jpg",
        }
    }
}

/// How many pixels a screenshot has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureResolution {
    /// One pixel per screen point, like Playwright's default `scale: "css"`. On a Retina
    /// display this has a quarter of the pixels of a full capture and stays readable.
    #[default]
    Points,
    /// Every physical pixel (twice the width and height of points on Retina displays).
    Pixels,
}

/// How a screenshot is captured and encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CaptureOptions {
    pub resolution: CaptureResolution,
    pub format: ImageFormat,
}

/// Encoded image bytes, with the format they are in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedImage {
    format: ImageFormat,
    bytes: Vec<u8>,
}

impl EncodedImage {
    pub fn new(format: ImageFormat, bytes: Vec<u8>) -> Self {
        Self { format, bytes }
    }

    pub fn format(&self) -> ImageFormat {
        self.format
    }

    pub fn mime_type(&self) -> &'static str {
        self.format.mime_type()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_jpeg_quality() {
        assert_eq!(
            ImageFormat::jpeg(None),
            Ok(ImageFormat::Jpeg { quality: 80 })
        );
        assert_eq!(
            ImageFormat::jpeg(Some(100)),
            Ok(ImageFormat::Jpeg { quality: 100 })
        );
        assert!(ImageFormat::jpeg(Some(0)).is_err());
        assert!(ImageFormat::jpeg(Some(101)).is_err());
    }

    #[test]
    fn names_mime_types_and_extensions() {
        let jpeg = ImageFormat::Jpeg { quality: 50 };

        assert_eq!(ImageFormat::Png.mime_type(), "image/png");
        assert_eq!(jpeg.mime_type(), "image/jpeg");
        assert_eq!(ImageFormat::Png.file_extension(), "png");
        assert_eq!(jpeg.file_extension(), "jpg");
    }

    #[test]
    fn defaults_to_point_resolution_png() {
        let options = CaptureOptions::default();

        assert_eq!(options.resolution, CaptureResolution::Points);
        assert_eq!(options.format, ImageFormat::Png);
    }

    #[test]
    fn keeps_bytes_and_format_together() {
        let image = EncodedImage::new(ImageFormat::Png, vec![1, 2, 3]);

        assert_eq!(image.as_bytes(), [1, 2, 3]);
        assert_eq!(image.format(), ImageFormat::Png);
        assert_eq!(image.mime_type(), "image/png");
    }
}
