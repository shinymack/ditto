use chrono::Utc;
use ditto_core::config::Config;
use ditto_core::db::{ClipboardItem, Db};
use parking_lot::{Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::search::search_items;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub history_cache: Arc<RwLock<Vec<ClipboardItem>>>,
    pub config: Arc<RwLock<Config>>,
    pub is_paused: Arc<AtomicBool>,
    pub last_shown: Arc<Mutex<Option<Instant>>>,
}

impl AppState {
    pub fn new(db: Db, config: Config) -> Self {
        let max_items = config.max_items;
        let initial_items = db.get_history(max_items, None).unwrap_or_default();

        Self {
            db,
            history_cache: Arc::new(RwLock::new(initial_items)),
            config: Arc::new(RwLock::new(config)),
            is_paused: Arc::new(AtomicBool::new(false)),
            last_shown: Arc::new(Mutex::new(None)),
        }
    }

    /// Instant <0.05ms search from RAM cache
    pub fn search(&self, query: &str, limit: usize) -> Vec<ClipboardItem> {
        let cache = self.history_cache.read();
        search_items(&cache, query, limit)
    }

    /// Ingests a new clipboard clip: updates SQLite and updates RAM cache
    pub fn insert_clip(&self, text: &str) -> rusqlite::Result<()> {
        let max_items = self.config.read().max_items;

        // 1. Durably update SQLite
        self.db.insert_or_update(text, max_items)?;

        // 2. Fetch or compute the current timestamp
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        // 3. Atomically update RAM cache
        let mut cache = self.history_cache.write();
        if let Some(pos) = cache.iter().position(|item| item.content == text) {
            let mut item = cache.remove(pos);
            item.created_at = now;
            cache.insert(0, item);
        } else {
            // Need a valid ID; fetch newest from DB or use max id + 1
            let new_id = cache.first().map(|i| i.id + 1).unwrap_or(1);
            cache.insert(
                0,
                ClipboardItem {
                    id: new_id,
                    content: text.to_string(),
                    created_at: now,
                },
            );
        }

        if cache.len() > max_items {
            cache.truncate(max_items);
        }

        Ok(())
    }

    /// Deletes single item from SQLite and RAM cache
    pub fn delete_clip(&self, id: i64) -> rusqlite::Result<()> {
        self.db.delete_item(id)?;
        let mut cache = self.history_cache.write();
        cache.retain(|item| item.id != id);
        Ok(())
    }

    /// Clears all clipboard items from SQLite and RAM cache
    pub fn clear_history(&self) -> rusqlite::Result<()> {
        self.db.clear_history()?;
        let mut cache = self.history_cache.write();
        cache.clear();
        Ok(())
    }

    /// Checks if monitoring is paused
    pub fn is_paused(&self) -> bool {
        self.is_paused.load(Ordering::SeqCst)
    }

    /// Sets monitoring pause state
    pub fn set_paused(&self, paused: bool) {
        self.is_paused.store(paused, Ordering::SeqCst);
    }

    /// Saves updated config to disk and updates memory
    pub fn save_config(&self, new_config: Config) -> std::io::Result<()> {
        new_config.save()?;
        *self.config.write() = new_config;
        Ok(())
    }

    /// Re-reads all items from SQLite to ensure cache coherence if needed
    pub fn reload_from_db(&self) {
        let max_items = self.config.read().max_items;
        if let Ok(items) = self.db.get_history(max_items, None) {
            *self.history_cache.write() = items;
        }
    }
}
