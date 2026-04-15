//! Image decoding, resizing, re-encoding — the privacy-critical path.
//!
//! The key contract: raw bytes that may carry EXIF/GPS/IPTC metadata are
//! parsed into an in-memory RGBA buffer and never written back out. The
//! re-encoded WebP contains pixel data only.
//!
//! Used by:
//! - avatar upload (`ResizeMode::CenterSquare`, 64/256/512)
//! - post & story images (`ResizeMode::FitWithin`, 256/1080/2048)
//! - snap media (`ResizeMode::FitWithin`, 2048)

use std::io::Cursor;

use image::{imageops::FilterType, ImageReader};

use crate::error::{ApiError, ApiResult};

/// How to derive a variant from the source image.
#[derive(Debug, Clone, Copy)]
pub enum ResizeMode {
    /// Centre-crop to a square, then resize to `size × size`.
    CenterSquare,
    /// Keep aspect ratio; resize so the longest side is `size`.
    FitWithin,
}

#[derive(Debug, Clone, Copy)]
pub struct ImageVariant {
    pub name: &'static str,
    pub size: u32,
    pub mode: ResizeMode,
}

#[derive(Debug)]
pub struct ProcessedVariant {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Decode `bytes` once, then encode every variant as WebP.
///
/// Runs on whatever tokio thread calls it. Callers that are on an async
/// request should wrap in `tokio::task::spawn_blocking` because image
/// decoding is CPU-bound and can stall the reactor.
pub fn process_image(bytes: &[u8], variants: &[ImageVariant]) -> ApiResult<Vec<ProcessedVariant>> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| ApiError::bad_request("bad_image", e.to_string()))?;
    let img = reader
        .decode()
        .map_err(|e| ApiError::bad_request("bad_image", e.to_string()))?;

    // Normalise to RGBA up-front. Decoding doesn't carry EXIF orientation
    // metadata through to the buffer; the pixel data is whatever the file
    // visually represents.
    let rgba = img.to_rgba8();
    let source = image::DynamicImage::ImageRgba8(rgba);

    let mut out = Vec::with_capacity(variants.len());
    for v in variants {
        let resized = match v.mode {
            ResizeMode::CenterSquare => {
                let (w, h) = (source.width(), source.height());
                let side = w.min(h);
                let x = (w - side) / 2;
                let y = (h - side) / 2;
                let square = image::imageops::crop_imm(&source, x, y, side, side).to_image();
                image::DynamicImage::ImageRgba8(square)
                    .resize_exact(v.size, v.size, FilterType::Lanczos3)
            }
            ResizeMode::FitWithin => source.resize(v.size, v.size, FilterType::Lanczos3),
        };
        let rgba = resized.to_rgba8();
        let encoder = webp::Encoder::from_rgba(rgba.as_raw(), resized.width(), resized.height());
        let webp = encoder.encode(80.0);
        out.push(ProcessedVariant {
            name: v.name,
            bytes: webp.to_vec(),
            width: resized.width(),
            height: resized.height(),
        });
    }
    Ok(out)
}

/// Convenience: variants for user avatars (square, 3 sizes).
pub const AVATAR_VARIANTS: &[ImageVariant] = &[
    ImageVariant { name: "thumb", size: 64, mode: ResizeMode::CenterSquare },
    ImageVariant { name: "medium", size: 256, mode: ResizeMode::CenterSquare },
    ImageVariant { name: "full", size: 512, mode: ResizeMode::CenterSquare },
];

/// Variants for post/story images (fit, 3 sizes).
pub const POST_VARIANTS: &[ImageVariant] = &[
    ImageVariant { name: "thumb", size: 256, mode: ResizeMode::FitWithin },
    ImageVariant { name: "medium", size: 1080, mode: ResizeMode::FitWithin },
    ImageVariant { name: "full", size: 2048, mode: ResizeMode::FitWithin },
];
