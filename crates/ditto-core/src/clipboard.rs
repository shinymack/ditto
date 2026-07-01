use arboard::Clipboard;
use base64::prelude::*;
use image::{ImageBuffer, Rgba};
use std::io::Cursor;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

const DEFAULT_MAX_CLIP_SIZE: usize = 100 * 1024;

fn image_to_base64(img: arboard::ImageData) -> Option<String> {
    let width = img.width as u32;
    let height = img.height as u32;
    let buffer: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(width, height, img.bytes.into_owned())?;
    let mut bytes = Vec::new();
    buffer
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .ok()?;
    let b64 = BASE64_STANDARD.encode(&bytes);
    Some(format!("data:image/png;base64,{}", b64))
}

pub fn start_monitor(
    sender: Sender<String>,
    poll_interval_ms: u64,
    max_size: Option<usize>,
) -> thread::JoinHandle<()> {
    let limit = max_size.unwrap_or(DEFAULT_MAX_CLIP_SIZE);

    thread::spawn(move || {
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut last_clip = if let Ok(text) = clipboard.get_text() {
            text.trim().to_string()
        } else if let Ok(img) = clipboard.get_image() {
            image_to_base64(img).unwrap_or_default()
        } else {
            String::new()
        };

        loop {
            thread::sleep(Duration::from_millis(poll_interval_ms));

            if let Ok(current_text) = clipboard.get_text() {
                let trimmed = current_text.trim();
                if !trimmed.is_empty() && trimmed.len() <= limit && trimmed != last_clip {
                    last_clip = trimmed.to_string();
                    if sender.send(last_clip.clone()).is_err() {
                        break;
                    }
                }
            } else if let Ok(img) = clipboard.get_image() {
                if let Some(b64) = image_to_base64(img) {
                    if b64 != last_clip {
                        last_clip = b64;
                        if sender.send(last_clip.clone()).is_err() {
                            break;
                        }
                    }
                }
            }
        }
    })
}

pub fn set_text(content: &str) -> Result<(), arboard::Error> {
    let mut clipboard = Clipboard::new()?;
    if content.starts_with("data:image/png;base64,") {
        let b64_data = content.trim_start_matches("data:image/png;base64,");
        if let Ok(bytes) = BASE64_STANDARD.decode(b64_data) {
            if let Ok(img_buf) =
                image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            {
                let rgba = img_buf.to_rgba8();
                let width = rgba.width() as usize;
                let height = rgba.height() as usize;
                let raw_bytes = rgba.into_raw();
                let img_data = arboard::ImageData {
                    width,
                    height,
                    bytes: std::borrow::Cow::Owned(raw_bytes),
                };
                return clipboard.set_image(img_data);
            }
        }
    }
    clipboard.set_text(content.to_string())
}
