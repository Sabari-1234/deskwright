use std::io::Cursor;

use deskwright_core::model::encoded_image::{
    CaptureOptions, CaptureResolution, EncodedImage, ImageFormat,
};
use deskwright_core::{DeskwrightError, DeskwrightResult};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat as CodecFormat, RgbaImage};

/// Scales a captured image to the requested resolution, then encodes it. `point_size` is
/// the captured area's size in screen points; a Retina capture has twice as many pixels.
pub fn encode(
    image: RgbaImage,
    point_size: (u32, u32),
    options: &CaptureOptions,
) -> DeskwrightResult<EncodedImage> {
    let image = match options.resolution {
        CaptureResolution::Points => to_point_size(image, point_size),
        CaptureResolution::Pixels => image,
    };
    let mut bytes = Vec::new();
    let result = match options.format {
        ImageFormat::Png => image.write_to(&mut Cursor::new(&mut bytes), CodecFormat::Png),
        // JPEG has no alpha channel; screenshots are opaque anyway.
        ImageFormat::Jpeg { quality } => JpegEncoder::new_with_quality(&mut bytes, quality)
            .encode_image(&DynamicImage::ImageRgba8(image).to_rgb8()),
    };
    result.map_err(|error| {
        DeskwrightError::operation_failed("encode the image", error.to_string())
    })?;
    Ok(EncodedImage::new(options.format, bytes))
}

/// Downscales to `point_size` when the image is larger (Retina); never upscales.
fn to_point_size(image: RgbaImage, (point_width, point_height): (u32, u32)) -> RgbaImage {
    let is_larger = image.width() > point_width || image.height() > point_height;
    if !is_larger || point_width == 0 || point_height == 0 {
        return image;
    }
    // Triangle (bilinear) is several times faster than Lanczos and keeps a halving sharp.
    image::imageops::resize(&image, point_width, point_height, FilterType::Triangle)
}

#[cfg(test)]
mod tests {
    use image::{Rgba, load_from_memory};

    use super::*;

    const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);

    fn options(resolution: CaptureResolution, format: ImageFormat) -> CaptureOptions {
        CaptureOptions { resolution, format }
    }

    fn decoded_size(image: &EncodedImage) -> (u32, u32) {
        let decoded = load_from_memory(image.as_bytes()).unwrap();
        (decoded.width(), decoded.height())
    }

    #[test]
    fn halves_a_retina_capture_to_points() {
        let retina = RgbaImage::from_pixel(8, 4, RED);

        let encoded = encode(
            retina,
            (4, 2),
            &options(CaptureResolution::Points, ImageFormat::Png),
        )
        .unwrap();

        assert_eq!(decoded_size(&encoded), (4, 2));
        let pixel = load_from_memory(encoded.as_bytes()).unwrap().to_rgba8();
        assert_eq!(pixel.get_pixel(3, 1), &RED);
    }

    #[test]
    fn keeps_every_pixel_when_asked_and_never_upscales() {
        let retina = RgbaImage::from_pixel(8, 4, RED);
        let standard = RgbaImage::from_pixel(4, 2, RED);

        let pixels = encode(
            retina,
            (4, 2),
            &options(CaptureResolution::Pixels, ImageFormat::Png),
        )
        .unwrap();
        let points = encode(
            standard,
            (4, 2),
            &options(CaptureResolution::Points, ImageFormat::Png),
        )
        .unwrap();

        assert_eq!(decoded_size(&pixels), (8, 4));
        assert_eq!(decoded_size(&points), (4, 2));
    }

    #[test]
    fn encodes_jpeg_with_its_format() {
        let image = RgbaImage::from_pixel(16, 16, RED);

        let jpeg = encode(
            image,
            (16, 16),
            &options(CaptureResolution::Points, ImageFormat::Jpeg { quality: 70 }),
        )
        .unwrap();

        assert_eq!(jpeg.format(), ImageFormat::Jpeg { quality: 70 });
        assert_eq!(&jpeg.as_bytes()[..2], [0xFF, 0xD8]);
        assert_eq!(decoded_size(&jpeg), (16, 16));
    }
}
