use arboard::Clipboard;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

const DEFAULT_MAX_CLIP_SIZE: usize = 100 * 1024;

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

        let mut last_text = clipboard
            .get_text()
            .map(|t| t.trim().to_string())
            .unwrap_or_default();

        loop {
            thread::sleep(Duration::from_millis(poll_interval_ms));

            if let Ok(current_text) = clipboard.get_text() {
                let trimmed = current_text.trim();
                if !trimmed.is_empty() && trimmed.len() <= limit && trimmed != last_text {
                    last_text = trimmed.to_string();
                    if sender.send(last_text.clone()).is_err() {
                        break;
                    }
                }
            }
        }
    })
}

pub fn set_text(content: &str) -> Result<(), arboard::Error> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(content.to_string())
}
