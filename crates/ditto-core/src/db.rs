use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

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
            conn.execute("PRAGMA user_version = 1", [])?;
        }

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn insert_or_update(&self, content: &str, max_items: usize) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO history (content, created_at)
             VALUES (?1, strftime('%Y-%m-%d %H:%M:%f', 'now'))
             ON CONFLICT(content) DO UPDATE SET created_at = strftime('%Y-%m-%d %H:%M:%f', 'now')",
            params![content],
        )?;

        // Enforce max_items limit by deleting older entries
        conn.execute(
            "DELETE FROM history WHERE id NOT IN (
                SELECT id FROM history ORDER BY created_at DESC LIMIT ?1
            )",
            params![max_items as i64],
        )?;

        Ok(())
    }

    pub fn get_history(&self, limit: usize, query: Option<&str>) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
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
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM history", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        db.clear_history().unwrap();
        let history = db.get_history(10, None).unwrap();
        assert_eq!(history.len(), 0);
    }
}
