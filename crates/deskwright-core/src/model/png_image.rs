/// PNG-encoded image bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PngImage(Vec<u8>);

impl PngImage {
    pub fn from_encoded_bytes(png_bytes: Vec<u8>) -> Self {
        Self(png_bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
