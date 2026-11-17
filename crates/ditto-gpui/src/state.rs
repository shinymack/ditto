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
    pub history_cache: Arc<RwLock<Vec<Arc<ClipboardItem>>>>,
    // Serializes SQLite/cache mutations without blocking RAM searches during disk I/O.
    history_mutation: Arc<Mutex<()>>,
    pub config: Arc<RwLock<Config>>,
    pub is_paused: Arc<AtomicBool>,
    pub last_shown: Arc<Mutex<Option<Instant>>>,
}

impl AppState {
    pub fn new(db: Db, config: Config) -> Self {
        let max_items = config.max_items;
        let initial_items = db
            .get_history(max_items, None)
            .unwrap_or_default()
            .into_iter()
            .map(Arc::new)
            .collect();

        Self {
            db,
            history_cache: Arc::new(RwLock::new(initial_items)),
            history_mutation: Arc::new(Mutex::new(())),
            config: Arc::new(RwLock::new(config)),
            is_paused: Arc::new(AtomicBool::new(false)),
            last_shown: Arc::new(Mutex::new(None)),
        }
    }

    /// Searches the RAM cache, sharing clipboard payloads with the returned snapshot.
    pub fn search(&self, query: &str, limit: usize) -> Vec<Arc<ClipboardItem>> {
        let cache = self.history_cache.read();
        search_items(&cache, query, limit)
    }

    /// Ingests a new clipboard clip: updates SQLite and updates RAM cache
    pub fn insert_clip(&self, text: &str) -> rusqlite::Result<()> {
        let _mutation = self.history_mutation.lock();
        let max_items = self.config.read().max_items;
        let item = Arc::new(self.db.insert_or_update(text, max_items)?);

        let mut cache = self.history_cache.write();
        if let Some(pos) = cache.iter().position(|cached| cached.id == item.id) {
            cache.remove(pos);
        }
        cache.insert(0, item);

        if cache.len() > max_items {
            cache.truncate(max_items);
        }

        Ok(())
    }

    /// Deletes single item from SQLite and RAM cache
    pub fn delete_clip(&self, id: i64) -> rusqlite::Result<()> {
        let _mutation = self.history_mutation.lock();
        self.db.delete_item(id)?;
        let mut cache = self.history_cache.write();
        cache.retain(|item| item.id != id);
        Ok(())
    }

    /// Clears all clipboard items from SQLite and RAM cache
    pub fn clear_history(&self) -> rusqlite::Result<()> {
        let _mutation = self.history_mutation.lock();
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
        let _mutation = self.history_mutation.lock();
        let max_items = self.config.read().max_items;
        if let Ok(items) = self.db.get_history(max_items, None) {
            let items = items.into_iter().map(Arc::new).collect();
            *self.history_cache.write() = items;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_cached_identities_match_db(state: &AppState) {
        let mut cached: Vec<_> = state
            .search("", 100)
            .into_iter()
            .map(|item| (item.id, item.content.clone(), item.created_at.clone()))
            .collect();
        let mut persisted: Vec<_> = state
            .db
            .get_history(100, None)
            .unwrap()
            .into_iter()
            .map(|item| (item.id, item.content, item.created_at))
            .collect();
        cached.sort();
        persisted.sort();
        assert_eq!(cached, persisted);
    }

    #[test]
    fn test_insert_after_reordered_duplicate_deletes_correct_id() {
        let state = AppState::new(Db::init(":memory:").unwrap(), Config::default());
        state.insert_clip("first").unwrap();
        let first_id = state.search("first", 1)[0].id;
        state.insert_clip("second").unwrap();
        let second_id = state.search("second", 1)[0].id;

        // The newest cache entry now has a lower ID than the second entry.
        state.insert_clip("first").unwrap();
        let reordered = state.search("", 10);
        assert_eq!(reordered[0].id, first_id);
        assert_eq!(reordered[1].id, second_id);
        assert!(first_id < second_id);

        state.insert_clip("fresh").unwrap();
        let fresh_id = state.search("fresh", 1)[0].id;
        assert!(fresh_id > second_id);
        assert_cached_identities_match_db(&state);

        state.delete_clip(fresh_id).unwrap();
        assert!(state.search("fresh", 1).is_empty());
        assert_eq!(state.search("first", 1)[0].id, first_id);
        assert_eq!(state.search("second", 1)[0].id, second_id);
        assert_cached_identities_match_db(&state);

        // AUTOINCREMENT does not restart when the RAM cache and table are cleared.
        state.clear_history().unwrap();
        state.insert_clip("after clear").unwrap();
        assert!(state.search("", 1)[0].id > fresh_id);
        assert_cached_identities_match_db(&state);
    }

    #[test]
    fn search_snapshot_survives_cache_update_reload_and_deletion() {
        let state = AppState::new(Db::init(":memory:").unwrap(), Config::default());
        state.insert_clip("snapshot content").unwrap();
        let snapshot = state.search("", 10);
        let original = &snapshot[0];
        let original_timestamp = original.created_at.clone();
        assert!(Arc::ptr_eq(original, &state.search("snapshot", 1)[0]));

        state.insert_clip("snapshot content").unwrap();
        let updated = state.search("snapshot", 1);
        assert_eq!(updated[0].id, original.id);
        assert!(!Arc::ptr_eq(original, &updated[0]));
        assert_eq!(original.created_at, original_timestamp);
        assert_cached_identities_match_db(&state);

        state.reload_from_db();
        let reloaded = state.search("snapshot", 1);
        assert_eq!(reloaded[0].id, original.id);
        assert_eq!(reloaded[0].content, original.content);
        assert_cached_identities_match_db(&state);

        state.delete_clip(original.id).unwrap();
        assert!(state.search("", 10).is_empty());
        assert_eq!(original.content, "snapshot content");
        assert_eq!(updated[0].content, "snapshot content");
        assert_eq!(reloaded[0].content, "snapshot content");
        assert_cached_identities_match_db(&state);
    }

    #[test]
    fn test_concurrent_mutations_keep_cached_identities_coherent() {
        let state = AppState::new(Db::init(":memory:").unwrap(), Config::default());
        let start = Arc::new(std::sync::Barrier::new(4));
        std::thread::scope(|scope| {
            for worker in 0..4 {
                let state = state.clone();
                let start = start.clone();
                scope.spawn(move || {
                    start.wait();
                    for iteration in 0..10 {
                        let content = format!("worker {worker} clip {iteration}");
                        state.insert_clip(&content).unwrap();
                        if let Some(item) = state.search(&content, 1).first() {
                            state.delete_clip(item.id).unwrap();
                        }
                        state.reload_from_db();
                        state.clear_history().unwrap();
                        state.insert_clip(&content).unwrap();
                    }
                });
            }
        });
        assert_cached_identities_match_db(&state);
    }
}
