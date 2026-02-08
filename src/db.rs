use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub parent_id: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl Node {
    pub fn new(content: String, parent_id: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            parent_id,
            content,
            created_at: Utc::now(),
        }
    }
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open() -> Result<Self> {
        let path = Self::default_path()?;
        Self::open_at(path)
    }

    fn default_path() -> Result<PathBuf> {
        let data_dir = dirs::data_local_dir()
            .context("Could not find local data directory")?
            .join("neuron");
        std::fs::create_dir_all(&data_dir)?;
        Ok(data_dir.join("neuron.db"))
    }

    pub fn open_at(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(&path)?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                parent_id TEXT,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_created ON nodes(created_at DESC);
            "#,
        )?;
        self.setup_fts()?;
        Ok(())
    }

    fn setup_fts(&self) -> Result<()> {
        let exists: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='nodes_fts'",
            [],
            |r| r.get(0),
        )?;

        if exists > 0 {
            if self
                .conn
                .query_row("SELECT COUNT(*) FROM nodes_fts LIMIT 1", [], |_| Ok(()))
                .is_ok()
            {
                return Ok(());
            }
            let _ = self.conn.execute_batch(
                "DROP TRIGGER IF EXISTS nodes_ai; DROP TRIGGER IF EXISTS nodes_ad; DROP TRIGGER IF EXISTS nodes_au; DROP TABLE IF EXISTS nodes_fts;",
            );
        }

        self.conn.execute_batch(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS nodes_fts USING fts5(content, content='nodes', content_rowid='rowid');

            CREATE TRIGGER IF NOT EXISTS nodes_ai AFTER INSERT ON nodes BEGIN
                INSERT INTO nodes_fts(rowid, content) VALUES (new.rowid, new.content);
            END;

            CREATE TRIGGER IF NOT EXISTS nodes_ad AFTER DELETE ON nodes BEGIN
                INSERT INTO nodes_fts(nodes_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
            END;

            CREATE TRIGGER IF NOT EXISTS nodes_au AFTER UPDATE ON nodes BEGIN
                INSERT INTO nodes_fts(nodes_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
                INSERT INTO nodes_fts(rowid, content) VALUES (new.rowid, new.content);
            END;
            "#,
        )?;

        self.conn.execute(
            "INSERT OR IGNORE INTO nodes_fts(rowid, content) SELECT rowid, content FROM nodes",
            [],
        )?;

        Ok(())
    }

    pub fn insert_node(&self, node: &Node) -> Result<()> {
        self.conn.execute(
            "INSERT INTO nodes (id, parent_id, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![node.id, node.parent_id, node.content, node.created_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn delete_node(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM nodes WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_recent(&self, limit: usize) -> Result<Vec<Node>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, parent_id, content, created_at FROM nodes ORDER BY created_at DESC LIMIT ?1",
        )?;
        let nodes = stmt
            .query_map(params![limit as i64], |row| {
                Ok(Node {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    content: row.get(2)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(nodes)
    }

    pub fn search(&self, query: &str) -> Result<Vec<Node>> {
        if query.trim().is_empty() {
            return self.get_recent(50);
        }

        let search_query = format!("{}*", query.replace('"', ""));

        let mut stmt = self.conn.prepare(
            r#"
            SELECT n.id, n.parent_id, n.content, n.created_at
            FROM nodes n
            JOIN nodes_fts ON n.rowid = nodes_fts.rowid
            WHERE nodes_fts MATCH ?1
            ORDER BY bm25(nodes_fts)
            LIMIT 50
            "#,
        )?;

        let nodes = stmt
            .query_map(params![search_query], |row| {
                Ok(Node {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    content: row.get(2)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(nodes)
    }
}
