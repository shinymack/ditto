use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: i64,
    pub content: String,
    pub created_at: String,
}

pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn init(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT UNIQUE NOT NULL,
                created_at TEXT DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
            )",
            [],
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn insert_or_update(&self, content: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO history (content, created_at)
             VALUES (?1, strftime('%Y-%m-%d %H:%M:%f', 'now'))
             ON CONFLICT(content) DO UPDATE SET created_at = strftime('%Y-%m-%d %H:%M:%f', 'now')",
            params![content],
        )?;
        Ok(())
    }

    pub fn get_history(&self, limit: usize) -> Result<Vec<ClipboardItem>> {
        let conn = self.conn.lock().unwrap();
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

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_operations() {
        let db = Db::init(":memory:").unwrap();

        db.insert_or_update("hello").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        db.insert_or_update("world").unwrap();

        let history = db.get_history(10).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "world");
        assert_eq!(history[1].content, "hello");

        std::thread::sleep(std::time::Duration::from_millis(2));
        db.insert_or_update("hello").unwrap();
        let history = db.get_history(10).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "hello");
        assert_eq!(history[1].content, "world");
    }
}
