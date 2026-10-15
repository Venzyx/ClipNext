use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use image::imageops::{self, FilterType};
use image::{ImageFormat, RgbaImage};
use sha2::{Digest, Sha256};

pub enum Captured {
    Text(String),
    Image {
        width: usize,
        height: usize,
        rgba: Vec<u8>,
    },
}

pub struct StoredImage {
    pub image_path: PathBuf,
    pub thumb_path: PathBuf,
    /// Size of the full image plus its thumbnail on disk.
    pub bytes: i64,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn hash_text(text: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"text:");
    h.update(text.as_bytes());
    hex(&h.finalize())
}

pub fn hash_image(width: usize, height: usize, rgba: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(format!("image:{width}x{height}:").as_bytes());
    h.update(rgba);
    hex(&h.finalize())
}

/// Reads the current clipboard. The clipboard is often locked for a few
/// milliseconds right after a change event, so retry briefly.
pub fn read() -> Option<Captured> {
    for _ in 0..5 {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            if let Ok(text) = cb.get_text() {
                if !text.trim().is_empty() {
                    return Some(Captured::Text(text));
                }
            }
            if let Ok(img) = cb.get_image() {
                return Some(Captured::Image {
                    width: img.width,
                    height: img.height,
                    rgba: img.bytes.into_owned(),
                });
            }
        }
        sleep(Duration::from_millis(40));
    }
    None
}

fn encode_png(img: &RgbaImage) -> Option<Vec<u8>> {
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .ok()?;
    Some(buf)
}

fn fit(img: &RgbaImage, max_edge: u32) -> RgbaImage {
    let longest = img.width().max(img.height());
    if longest <= max_edge {
        return img.clone();
    }
    let scale = max_edge as f32 / longest as f32;
    let w = ((img.width() as f32 * scale).round() as u32).max(1);
    let h = ((img.height() as f32 * scale).round() as u32).max(1);
    imageops::resize(img, w, h, FilterType::Triangle)
}

/// Saves the image as PNG (downscaled to `max_edge`) plus a small thumbnail.
/// Returns `None` if it cannot be encoded or the PNG exceeds `max_bytes`.
/// Files are named after the content hash, so identical images are stored once.
pub fn store_image(
    dir: &Path,
    hash: &str,
    width: usize,
    height: usize,
    rgba: Vec<u8>,
    max_bytes: usize,
    max_edge: u32,
    thumb_edge: u32,
) -> Option<StoredImage> {
    let image_path = dir.join(format!("{hash}.png"));
    let thumb_path = dir.join(format!("{hash}_t.png"));

    if let (Ok(a), Ok(b)) = (fs::metadata(&image_path), fs::metadata(&thumb_path)) {
        return Some(StoredImage {
            bytes: (a.len() + b.len()) as i64,
            image_path,
            thumb_path,
        });
    }

    let img = RgbaImage::from_raw(width as u32, height as u32, rgba)?;
    let full = fit(&img, max_edge);
    let png = encode_png(&full)?;
    if png.len() > max_bytes {
        return None;
    }
    let thumb_png = encode_png(&fit(&full, thumb_edge))?;

    fs::create_dir_all(dir).ok()?;
    fs::write(&image_path, &png).ok()?;
    fs::write(&thumb_path, &thumb_png).ok()?;
    Some(StoredImage {
        bytes: (png.len() + thumb_png.len()) as i64,
        image_path,
        thumb_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: usize, h: usize) -> Vec<u8> {
        vec![200; w * h * 4]
    }

    #[test]
    fn stores_image_and_thumbnail_once() {
        let dir = tempfile::tempdir().unwrap();
        let a = store_image(dir.path(), "abc", 8, 4, solid(8, 4), 1 << 20, 3840, 256).unwrap();
        assert!(a.image_path.exists() && a.thumb_path.exists());
        assert!(a.bytes > 0);
        let b = store_image(dir.path(), "abc", 8, 4, solid(8, 4), 1 << 20, 3840, 256).unwrap();
        assert_eq!(a.bytes, b.bytes);
    }

    #[test]
    fn downscales_large_images_and_makes_small_thumbnails() {
        let dir = tempfile::tempdir().unwrap();
        let s = store_image(dir.path(), "big", 400, 200, solid(400, 200), 1 << 20, 100, 20).unwrap();
        let full = image::open(&s.image_path).unwrap();
        let thumb = image::open(&s.thumb_path).unwrap();
        assert_eq!((full.width(), full.height()), (100, 50));
        assert_eq!((thumb.width(), thumb.height()), (20, 10));
    }

    #[test]
    fn rejects_images_over_the_size_limit() {
        let dir = tempfile::tempdir().unwrap();
        let noisy: Vec<u8> = (0..64 * 64 * 4).map(|i| ((i * 7919) % 251) as u8).collect();
        assert!(store_image(dir.path(), "n", 64, 64, noisy, 100, 3840, 256).is_none());
        assert!(!dir.path().join("n.png").exists());
    }

    #[test]
    fn hashes_are_stable_and_distinguish_kinds() {
        assert_eq!(hash_text("a"), hash_text("a"));
        assert_ne!(hash_text("a"), hash_text("b"));
        assert_ne!(hash_image(1, 1, &[0, 0, 0, 0]), hash_image(2, 1, &[0, 0, 0, 0]));
    }
}
