use crate::error::Result;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct Db {
    conn: Mutex<Connection>,
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct Admin {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

impl Db {
    pub fn init(data_path: &str) -> Result<Arc<Db>> {
        let db_path = std::path::Path::new(data_path).join("dockops.db");
        let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
        // Equivalent of the Go DSN `_pragma=foreign_keys(1)&_pragma=journal_mode(WAL)`.
        let _ = conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;");
        let db = Arc::new(Db {
            conn: Mutex::new(conn),
        });
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let queries = [
            "CREATE TABLE IF NOT EXISTS admin (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
        ];
        for q in queries {
            if let Err(e) = conn.execute(q, []) {
                if !e.to_string().contains("duplicate column") {
                    return Err(e.to_string());
                }
            }
        }
        let defaults = [
            ("update_check_interval", "6h"),
            ("docker_proxy", ""),
            ("collect_interval", "10m"),
        ];
        for (k, v) in defaults {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![k, v],
            );
        }
        Ok(())
    }

    pub fn is_setup(&self) -> bool {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM admin", [], |r| r.get::<_, i64>(0))
            .map(|c| c > 0)
            .unwrap_or(false)
    }

    pub fn get_setting(&self, key: &str) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| {
            r.get::<_, String>(0)
        })
        .map_err(|e| e.to_string())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    pub fn get_all_settings(&self) -> Result<HashMap<String, String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        let mut result = HashMap::new();
        for row in rows {
            let (k, v) = row.map_err(|e| e.to_string())?;
            result.insert(k, v);
        }
        Ok(result)
    }

    pub fn insert_admin(&self, username: &str, password_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO admin (username, password_hash) VALUES (?1, ?2)",
            params![username, password_hash],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    pub fn update_admin(&self, username: &str, password_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE admin SET username = ?1, password_hash = ?2 WHERE id = 1",
            params![username, password_hash],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    pub fn find_admin_by_username(&self, username: &str) -> Option<Admin> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, username, password_hash FROM admin WHERE username = ?1",
            params![username],
            |r| {
                Ok(Admin {
                    id: r.get(0)?,
                    username: r.get(1)?,
                    password_hash: r.get(2)?,
                })
            },
        )
        .ok()
    }

    /// Go's GetAdmin always returns a non-nil *Admin (zero value on query error),
    /// so callers always see an entry (possibly empty). Mirror that behaviour.
    pub fn get_admin(&self) -> Admin {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, username, password_hash FROM admin WHERE id = 1",
            [],
            |r| {
                Ok(Admin {
                    id: r.get(0)?,
                    username: r.get(1)?,
                    password_hash: r.get(2)?,
                })
            },
        )
        .unwrap_or_default()
    }
}
