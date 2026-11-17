use parking_lot::Mutex;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: i64,
    pub content: String,
    pub created_at: String,
}

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn init(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Enable Write-Ahead Logging (WAL) and normal synchronous for fast, non-blocking disk I/O
        let _ = conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;");

        let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < 1 {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content TEXT UNIQUE NOT NULL,
                    created_at TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
                )",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_history_created_at ON history(created_at DESC)",
                [],
            )?;
            conn.execute("PRAGMA user_version = 1", [])?;
        } else {
            let _ = conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_history_created_at ON history(created_at DESC)",
                [],
            );
        }

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Returns the persisted identity and timestamp for either a new or existing clip.
    pub fn insert_or_update(&self, content: &str, max_items: usize) -> Result<ClipboardItem> {
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let item = transaction.query_row(
            "INSERT INTO history (content, created_at)
             VALUES (?1, strftime('%Y-%m-%d %H:%M:%f', 'now'))
             ON CONFLICT(content) DO UPDATE SET created_at = strftime('%Y-%m-%d %H:%M:%f', 'now')
             RETURNING id, content, created_at",
            params![content],
            |row| {
                Ok(ClipboardItem {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    created_at: row.get(2)?,
                })
            },
        )?;

        // Enforce max_items limit by deleting older entries
        transaction.execute(
            "DELETE FROM history WHERE id NOT IN (
                SELECT id FROM history ORDER BY created_at DESC LIMIT ?1
            )",
            params![max_items as i64],
        )?;

        transaction.commit()?;
        Ok(item)
    }

    pub fn get_history(&self, limit: usize, query: Option<&str>) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock();
        let mut items = Vec::new();
        if let Some(q) = query {
            if !q.trim().is_empty() {
                let mut stmt = conn.prepare(
                    "SELECT id, content, created_at FROM history WHERE content LIKE ?1 ORDER BY created_at DESC LIMIT ?2",
                )?;
                let search_term = format!("%{}%", q.trim());
                let rows = stmt.query_map(params![search_term, limit as i64], |row| {
                    Ok(ClipboardItem {
                        id: row.get(0)?,
                        content: row.get(1)?,
                        created_at: row.get(2)?,
                    })
                })?;
                for row in rows {
                    items.push(row?);
                }
                return Ok(items);
            }
        }

        let mut stmt = conn.prepare(
            "SELECT id, content, created_at FROM history ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(ClipboardItem {
                id: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }

    pub fn clear_history(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }

    pub fn delete_item(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM history WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn count_items(&self) -> Result<i64> {
        let conn = self.conn.lock();
        conn.query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_returns_persisted_identity_after_duplicate() {
        let db = Db::init(":memory:").unwrap();
        let first = db.insert_or_update("first", 10).unwrap();
        let second = db.insert_or_update("second", 10).unwrap();
        let duplicate = db.insert_or_update("first", 10).unwrap();
        let fresh = db.insert_or_update("fresh", 10).unwrap();

        assert_eq!(duplicate.id, first.id);
        assert!(fresh.id > second.id);
        assert_ne!(fresh.id, duplicate.id);
        let history = db.get_history(10, None).unwrap();
        for returned in [&duplicate, &second, &fresh] {
            let persisted = history.iter().find(|item| item.id == returned.id).unwrap();
            assert_eq!(persisted.content, returned.content);
            assert_eq!(persisted.created_at, returned.created_at);
        }

        db.delete_item(fresh.id).unwrap();
        let remaining = db.get_history(10, None).unwrap();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().any(|item| item.id == first.id));
        assert!(remaining.iter().any(|item| item.id == second.id));
        assert!(!remaining.iter().any(|item| item.id == fresh.id));
    }

    #[test]
    fn test_db_operations() {
        let db = Db::init(":memory:").unwrap();

        db.insert_or_update("hello", 10).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        db.insert_or_update("world", 10).unwrap();

        let history = db.get_history(10, None).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "world");
        assert_eq!(history[1].content, "hello");

        std::thread::sleep(std::time::Duration::from_millis(2));
        db.insert_or_update("hello", 10).unwrap();
        let history = db.get_history(10, None).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "hello");
        assert_eq!(history[1].content, "world");

        // Test search query filter
        let history = db.get_history(10, Some("he")).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, "hello");

        let to_delete_id = history[0].id;
        db.delete_item(to_delete_id).unwrap();
        let history = db.get_history(10, None).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].content, "world");
        db.clear_history().unwrap();
        let history = db.get_history(10, None).unwrap();
        assert_eq!(history.len(), 0);
    }
}
