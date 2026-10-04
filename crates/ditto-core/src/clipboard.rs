use base64::prelude::*;
use clipboard_rs::common::RustImage;
use clipboard_rs::{
    Clipboard, ClipboardContext, ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext,
};
use parking_lot::Mutex;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

struct ClipboardReceiver {
    ctx: ClipboardContext,
    sender: Sender<String>,
    limit: usize,
    last_clip: Arc<Mutex<String>>,
}

impl ClipboardHandler for ClipboardReceiver {
    fn on_clipboard_change(&mut self) {
        let mut processed_text = false;
        if let Ok(text) = self.ctx.get_text() {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                if trimmed.len() <= self.limit {
                    let mut last = self.last_clip.lock();
                    if *last != trimmed {
                        *last = trimmed.to_string();
                        let _ = self.sender.send(trimmed.to_string());
                    }
                }
                processed_text = true;
            }
        }

        if !processed_text {
            if let Ok(img) = self.ctx.get_image() {
                if let Ok(png_data) = img.to_png() {
                    let bytes = png_data.get_bytes();
                    let b64 = format!("data:image/png;base64,{}", BASE64_STANDARD.encode(bytes));
                    let mut last = self.last_clip.lock();
                    if *last != b64 {
                        *last = b64.clone();
                        let _ = self.sender.send(b64);
                    }
                }
            }
        }
    }
}

pub fn start_monitor(sender: Sender<String>, max_size: Option<usize>) -> thread::JoinHandle<()> {
    let limit = max_size.unwrap_or(100 * 1024);
    thread::spawn(move || {
        let ctx = match ClipboardContext::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        let receiver = ClipboardReceiver {
            ctx,
            sender,
            limit,
            last_clip: Arc::new(Mutex::new(String::new())),
        };
        let mut watcher = match ClipboardWatcherContext::new() {
            Ok(w) => w,
            Err(_) => return,
        };
        watcher.add_handler(receiver);
        watcher.start_watch();
    })
}

pub fn set_text(content: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ctx = ClipboardContext::new()?;
    if content.starts_with("data:image/png;base64,") {
        let b64_data = content.trim_start_matches("data:image/png;base64,");
        if let Ok(bytes) = BASE64_STANDARD.decode(b64_data) {
            if let Ok(img) = clipboard_rs::common::RustImage::from_bytes(&bytes) {
                return ctx.set_image(img);
            }
        }
    }
    ctx.set_text(content.to_string())
}
