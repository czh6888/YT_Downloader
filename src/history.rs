use rusqlite::{Connection, Result, params};
use std::path::PathBuf;

/// 下载历史记录条目。
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub format: String,
    pub status: String,
    pub date: String,
    pub file_path: String,
}

/// SQLite 下载历史管理器。
pub struct HistoryManager {
    conn: Connection,
}

fn db_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("yt-downloader").join("history.db"))
}

impl HistoryManager {
    /// 打开或创建数据库。
    pub fn new() -> Option<Self> {
        let path = db_path()?;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(&path).ok()?;
        let mut mgr = HistoryManager { conn };
        mgr.create_table().ok()?;
        Some(mgr)
    }

    fn create_table(&mut self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS downloads (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                url TEXT NOT NULL,
                format TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT '',
                date TEXT NOT NULL DEFAULT (datetime('now')),
                file_path TEXT NOT NULL DEFAULT ''
            )",
            [],
        )?;
        Ok(())
    }

    /// 添加一条下载记录，返回新记录的 id。
    pub fn add_entry(
        &self,
        title: &str,
        url: &str,
        format: &str,
        status: &str,
        file_path: &str,
    ) -> Result<i64> {
        let date = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        self.conn.execute(
            "INSERT INTO downloads (title, url, format, status, date, file_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![title, url, format, status, date, file_path],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// 加载所有历史记录，按时间倒序。
    pub fn load_entries(&self) -> Vec<HistoryEntry> {
        let stmt = self
            .conn
            .prepare(
                "SELECT id, title, url, format, status, date, file_path
                 FROM downloads ORDER BY id DESC",
            )
            .ok();

        let Some(mut stmt) = stmt else {
            return Vec::new();
        };

        let rows = stmt.query_map([], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                title: row.get(1)?,
                url: row.get(2)?,
                format: row.get(3)?,
                status: row.get(4)?,
                date: row.get(5)?,
                file_path: row.get(6)?,
            })
        }).ok();

        let Some(rows) = rows else {
            return Vec::new();
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// 搜索历史记录。
    pub fn search_entries(&self, query: &str) -> Vec<HistoryEntry> {
        let pattern = format!("%{query}%");
        let stmt = self
            .conn
            .prepare(
                "SELECT id, title, url, format, status, date, file_path
                 FROM downloads WHERE title LIKE ?1 OR url LIKE ?1 ORDER BY id DESC",
            )
            .ok();

        let Some(mut stmt) = stmt else {
            return Vec::new();
        };

        let rows = stmt.query_map(params![pattern], |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                title: row.get(1)?,
                url: row.get(2)?,
                format: row.get(3)?,
                status: row.get(4)?,
                date: row.get(5)?,
                file_path: row.get(6)?,
            })
        }).ok();

        let Some(rows) = rows else {
            return Vec::new();
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// 删除一条记录。
    pub fn delete_entry(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM downloads WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// 清空所有历史记录。
    pub fn clear_all(&self) -> Result<()> {
        self.conn.execute("DELETE FROM downloads", [])?;
        Ok(())
    }
}
