use std::io::Cursor;

use deskwright_core::model::png_image::PngImage;
use deskwright_core::{DeskwrightError, DeskwrightResult};
use xcap::image::{ImageFormat, RgbaImage};

pub fn encode_png(image: &RgbaImage) -> DeskwrightResult<PngImage> {
    let mut png_bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .map_err(|error| DeskwrightError::operation_failed("encode PNG", error.to_string()))?;
    Ok(PngImage::from_encoded_bytes(png_bytes))
}

#[cfg(test)]
mod tests {
    use xcap::image::{Rgba, load_from_memory};

    use super::*;

    #[test]
    fn encodes_pixels_as_a_decodable_png() {
        let red_pixel = Rgba([255, 0, 0, 255]);
        let image = RgbaImage::from_pixel(3, 2, red_pixel);

        let png = encode_png(&image).unwrap();

        let decoded = load_from_memory(png.as_bytes()).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (3, 2));
        assert_eq!(decoded.get_pixel(2, 1), &red_pixel);
    }
}
