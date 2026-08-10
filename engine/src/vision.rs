//! Image vision — lets ayesha "see" screenshots and image files using a
//! vision-capable model (local ollama like llama3.2-vision, or any cloud
//! OpenAI-compatible vision model such as cloudflare / gpt-4o / grok-4).

use anyhow::Result;
use base64::Engine;
use std::path::{Path, PathBuf};

use crate::cloud::CloudClient;
use crate::ollama::OllamaClient;

/// Convert an image file into a data URI (`data:<mime>;base64,...`) ready for
/// OpenAI-compatible image_url content parts.
pub fn image_data_uri(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let format = image::guess_format(&bytes)
        .map_err(|_| anyhow::anyhow!("unrecognized image format: {}", path.display()))?;
    encode_data_uri(&bytes, format)
}

/// Base64-encode raw image bytes into a `data:<mime>;base64,...` URI.
pub fn encode_data_uri(bytes: &[u8], format: image::ImageFormat) -> Result<String> {
    let mime = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::Gif => "image/gif",
        image::ImageFormat::WebP => "image/webp",
        image::ImageFormat::Bmp => "image/bmp",
        image::ImageFormat::Tiff => "image/tiff",
        _ => "image/png",
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{};base64,{}", mime, b64))
}

/// If the clipboard currently holds an image bitmap (e.g. Win+Shift+S), return
/// it as a PNG data URI. Clears the clipboard so Ctrl+V isn't hijacked twice.
pub fn clipboard_image_data_uri() -> Option<String> {
    let mut cb = arboard::Clipboard::new().ok()?;
    let img = cb.get_image().ok()?;
    let mut rgba = image::RgbaImage::new(img.width as u32, img.height as u32);
    for (x, y, px) in rgba.enumerate_pixels_mut() {
        let i = ((y as usize) * img.width as usize + x as usize) * 4;
        *px = image::Rgba([img.bytes[i], img.bytes[i + 1], img.bytes[i + 2], img.bytes[i + 3]]);
    }
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(rgba)
        .write_to(&mut out, image::ImageFormat::Png)
        .ok()?;
    encode_data_uri(&out.into_inner(), image::ImageFormat::Png).ok()
}

/// Check whether a string looks like a path to an existing image file.
pub fn is_image_path(candidate: &str) -> bool {
    let trimmed = candidate.trim().trim_matches('"');
    if trimmed.is_empty() {
        return false;
    }
    let p = Path::new(trimmed);
    p.is_file() && image::open(p).is_ok()
}

/// Normalize a typed/dropped path: strip surrounding quotes, drop leading
/// "file://" schemes. Returns None if it doesn't exist.
pub fn resolve_path(candidate: &str) -> Option<PathBuf> {
    let mut t = candidate.trim().trim_matches('"').to_string();
    if let Some(stripped) = t.strip_prefix("file://") {
        t = stripped.to_string();
    }
    if t.starts_with("file:///") {
        t = t.replacen("file:///", "", 1);
    }
    let p = PathBuf::from(&t);
    p.exists().then_some(p)
}

/// Locate the most recent screenshot in the user's Screenshots folder.
pub fn latest_screenshot() -> Result<PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(home).join("Pictures").join("Screenshots");
    if !dir.is_dir() {
        anyhow::bail!("screenshots dir not found: {}", dir.display());
    }
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_lowercase();
            n.ends_with(".png")
                || n.ends_with(".jpg")
                || n.ends_with(".jpeg")
                || n.ends_with(".webp")
        })
        .collect();
    entries.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));
    entries
        .pop()
        .map(|e| e.path())
        .ok_or_else(|| anyhow::anyhow!("no screenshots found in {}", dir.display()))
}

/// Stream a vision description from a local ollama vision model.
pub async fn describe_ollama(
    model: &str,
    data_uri: &str,
    prompt: &str,
    steer_rx: &std::sync::mpsc::Receiver<String>,
) -> Result<crate::ollama::StreamResult> {
    OllamaClient::new(model)
        .chat_with_image(prompt, data_uri, steer_rx)
        .await
}

/// Stream a vision description from a cloud (OpenAI-compatible) vision model.
pub async fn describe_cloud(
    model: &str,
    provider: &str,
    data_uri: &str,
    prompt: &str,
    steer_rx: &std::sync::mpsc::Receiver<String>,
) -> Result<crate::ollama::StreamResult> {
    CloudClient::new(model, provider)?
        .chat_with_image(prompt, data_uri, steer_rx)
        .await
}

/// A short, neutral instruction for describing screenshots.
pub fn describe_prompt(question: &str) -> String {
    if question.trim().is_empty() {
        "describe this screenshot in detail: what is shown, the layout, colours, and any notable text or UI elements. be specific and concise.".to_string()
    } else {
        question.to_string()
    }
}

/// Default vision model preference chain: [user-selected or local, cloudflare, gpt-4o]
pub const DEFAULT_FALLBACKS: &[(&str, &str)] = &[
    ("llama3.2-vision", "ollama"),
    ("@cf/meta/llama-3.2-11b-vision-instruct", "cloudflare"),
    ("gpt-4o", "github"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_becomes_data_uri() {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&[0; 16]);
        let dir = std::env::temp_dir();
        let path = dir.join("ayesha_vision_test.png");
        std::fs::write(&path, &png).unwrap();
        let uri = image_data_uri(&path).unwrap();
        assert!(uri.starts_with("data:image/png;base64,"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn non_image_rejected() {
        let dir = std::env::temp_dir();
        let path = dir.join("ayesha_vision_test.bin");
        std::fs::write(&path, b"not an image at all").unwrap();
        assert!(image_data_uri(&path).is_err());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn quoted_paths_resolve() {
        let dir = std::env::temp_dir();
        let path = dir.join("ayesha path test.png");
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]));
        img.save(&path).unwrap();
        assert!(is_image_path(&format!("\"{}\"", path.display())));
        assert_eq!(
            resolve_path(&format!("\"{}\"", path.display())).unwrap(),
            path
        );
        assert!(resolve_path("C:/definitely/not/a/real/path.png").is_none());
        std::fs::remove_file(&path).ok();
    }
}
