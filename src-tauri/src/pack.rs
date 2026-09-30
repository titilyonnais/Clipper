//! Background work on stored images: lossless recompression and the small
//! thumbnails the lists show.
//!
//! Images are written as they come from the clipboard, quickly and often
//! poorly compressed. Later, at low priority, each one is re-encoded with the
//! strongest PNG compression, without the channels it does not use (alpha
//! when fully opaque, colour when grey). The result replaces the file only if
//! it is smaller and decodes to exactly the same pixels. A `tEXt` chunk marks
//! processed files, so the work is done once; the file keeps its name, which
//! the database and its backups refer to.

use crate::db::Db;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ExtendedColorType, ImageEncoder};
use std::path::Path;
use std::time::Duration;

const MARK_KEYWORD: &[u8] = b"Software";
const MARK_TEXT: &[u8] = b"Clipper";
/// Signature (8 bytes) and IHDR chunk (25 bytes): the mark follows them.
const AFTER_IHDR: usize = 33;
/// Side of the thumbnails, twice the size of a row's image at 100 % scale.
pub const THUMB_SIDE: u32 = 96;

/// Content of the mark chunk: keyword, separator, text.
fn mark_data() -> Vec<u8> {
    [MARK_KEYWORD, &[0], MARK_TEXT].concat()
}

fn is_marked(png: &[u8]) -> bool {
    let expected = [b"tEXt".as_slice(), &mark_data()].concat();
    png.get(AFTER_IHDR + 4..AFTER_IHDR + 4 + expected.len()) == Some(expected.as_slice())
}

/// The same PNG with the mark inserted after its header.
fn marked(png: &[u8]) -> Option<Vec<u8>> {
    if png.get(12..16) != Some(b"IHDR") || png.len() < AFTER_IHDR {
        return None;
    }
    let data = mark_data();
    let mut crc = crc32fast::Hasher::new();
    crc.update(b"tEXt");
    crc.update(&data);
    let mut out = Vec::with_capacity(png.len() + 12 + data.len());
    out.extend_from_slice(&png[..AFTER_IHDR]);
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(b"tEXt");
    out.extend_from_slice(&data);
    out.extend_from_slice(&crc.finalize().to_be_bytes());
    out.extend_from_slice(&png[AFTER_IHDR..]);
    Some(out)
}

/// Pixels in the smallest 8-bit layout that keeps them all.
fn reduced(img: &DynamicImage) -> (Vec<u8>, ExtendedColorType) {
    let rgba = img.to_rgba8();
    let opaque = rgba.pixels().all(|p| p[3] == 255);
    let grey = rgba.pixels().all(|p| p[0] == p[1] && p[1] == p[2]);
    let pick = |channels: &[usize]| -> Vec<u8> {
        rgba.pixels()
            .flat_map(|p| channels.iter().map(move |&c| p[c]))
            .collect()
    };
    match (grey, opaque) {
        (true, true) => (pick(&[0]), ExtendedColorType::L8),
        (true, false) => (pick(&[0, 3]), ExtendedColorType::La8),
        (false, true) => (pick(&[0, 1, 2]), ExtendedColorType::Rgb8),
        (false, false) => (rgba.into_raw(), ExtendedColorType::Rgba8),
    }
}

/// Recompress a PNG without loss. `None` when it is already processed or
/// cannot be read; otherwise the new (marked) content, which may be the
/// original bytes when no smaller encoding was found.
pub fn pack(png: &[u8]) -> Option<Vec<u8>> {
    if is_marked(png) {
        return None;
    }
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?;
    // 16-bit and float images would lose precision in an 8-bit layout.
    let eight_bit = matches!(
        img,
        DynamicImage::ImageLuma8(_)
            | DynamicImage::ImageLumaA8(_)
            | DynamicImage::ImageRgb8(_)
            | DynamicImage::ImageRgba8(_)
    );
    let smaller = eight_bit
        .then(|| {
            let (pixels, layout) = reduced(&img);
            let mut out = Vec::new();
            PngEncoder::new_with_quality(&mut out, CompressionType::Best, FilterType::Adaptive)
                .write_image(&pixels, img.width(), img.height(), layout)
                .ok()?;
            let same = image::load_from_memory_with_format(&out, image::ImageFormat::Png)
                .is_ok_and(|back| back.to_rgba8() == img.to_rgba8());
            (same && out.len() < png.len()).then_some(out)
        })
        .flatten();
    marked(smaller.as_deref().unwrap_or(png))
}

/// The centre square of the image, `THUMB_SIDE` wide at most: what a row
/// shows (its image is a cropped square).
pub fn thumbnail(png: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?;
    let side = img.width().min(img.height());
    if side == 0 {
        return None;
    }
    let square = img.crop_imm(
        (img.width() - side) / 2,
        (img.height() - side) / 2,
        side,
        side,
    );
    let small = if side > THUMB_SIDE {
        square.thumbnail_exact(THUMB_SIDE, THUMB_SIDE)
    } else {
        square
    };
    let (pixels, layout) = reduced(&small);
    let mut out = Vec::new();
    PngEncoder::new_with_quality(&mut out, CompressionType::Best, FilterType::Adaptive)
        .write_image(&pixels, small.width(), small.height(), layout)
        .ok()?;
    Some(out)
}

/// Replace `path` with `data` without ever leaving a half-written file.
fn replace(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Process every stored image not done yet. Runs at background priority
/// (processor and disk) and pauses between images to stay unnoticed.
/// Returns the number of thumbnails made (the lists can then use them).
pub fn run(db: &Db) -> usize {
    use windows_sys::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_MODE_BACKGROUND_BEGIN,
        THREAD_MODE_BACKGROUND_END,
    };
    let files = match db.image_files() {
        Ok(files) => files,
        Err(e) => {
            log::warn!("pack: {e}");
            return 0;
        }
    };
    let mut thumbs = 0;
    unsafe { SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_BEGIN) };
    for file in files {
        let path = db.image_path(&file);
        let Ok(png) = std::fs::read(&path) else {
            continue;
        };
        let thumb = db.thumb_path(&file);
        if !thumb.exists() {
            if let Some(small) = thumbnail(&png) {
                thumbs += usize::from(replace(&thumb, &small).is_ok());
            }
        }
        if let Some(packed) = pack(&png) {
            // The clip may have been deleted meanwhile: do not bring its file back.
            if path.exists() {
                if let Err(e) = replace(&path, &packed) {
                    log::warn!("pack {file}: {e}");
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    unsafe { SetThreadPriority(GetCurrentThread(), THREAD_MODE_BACKGROUND_END) };
    thumbs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_of(img: &DynamicImage) -> Vec<u8> {
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn packs_without_loss_and_only_once() {
        let img = DynamicImage::ImageRgba8(image::RgbaImage::from_fn(300, 200, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 40, 255])
        }));
        let original = png_of(&img);
        let packed = pack(&original).unwrap();
        assert!(is_marked(&packed));
        assert!(packed.len() < original.len());
        let back = image::load_from_memory(&packed).unwrap();
        assert_eq!(back.to_rgba8(), img.to_rgba8());
        assert!(pack(&packed).is_none());
    }

    #[test]
    fn keeps_transparency_and_grey() {
        let img = DynamicImage::ImageRgba8(image::RgbaImage::from_fn(64, 64, |x, y| {
            let v = ((x + y) * 2) as u8;
            image::Rgba([v, v, v, (x * 4) as u8])
        }));
        let packed = pack(&png_of(&img)).unwrap();
        assert_eq!(
            image::load_from_memory(&packed).unwrap().to_rgba8(),
            img.to_rgba8()
        );
    }

    #[test]
    fn thumbnails_are_small_squares() {
        let img = DynamicImage::ImageRgb8(image::RgbImage::new(1920, 1080));
        let thumb = image::load_from_memory(&thumbnail(&png_of(&img)).unwrap()).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (THUMB_SIDE, THUMB_SIDE));
        let tiny = DynamicImage::ImageRgb8(image::RgbImage::new(40, 10));
        let thumb = image::load_from_memory(&thumbnail(&png_of(&tiny)).unwrap()).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (10, 10));
    }
}
