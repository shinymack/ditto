use arboard::Clipboard;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

pub fn start_monitor(sender: Sender<String>, poll_interval_ms: u64) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut clipboard = match Clipboard::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut last_text = clipboard.get_text().unwrap_or_default();

        loop {
            thread::sleep(Duration::from_millis(poll_interval_ms));

            if let Ok(current_text) = clipboard.get_text() {
                if !current_text.is_empty() && current_text != last_text {
                    last_text = current_text.clone();
                    if sender.send(current_text).is_err() {
                        break;
                    }
                }
            }
        }
    })
}
