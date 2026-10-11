
use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use rusqlite::{params, Connection};

use crate::models::notification::{Notification, NotificationLevel};

#[derive(Clone)]
pub struct NotificationRepository {
    connection: Arc<Mutex<Connection>>,
}

impl NotificationRepository {
    pub fn new(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let connection = Connection::open(path).map_err(|e| e.to_string())?;

        connection
            .execute_batch(
                "
                PRAGMA journal_mode = WAL;

                CREATE TABLE IF NOT EXISTS notifications (
                    id INTEGER PRIMARY KEY,
                    app_name TEXT NOT NULL,
                    app_icon TEXT NOT NULL DEFAULT '',
                    summary TEXT NOT NULL,
                    body TEXT NOT NULL DEFAULT '',
                    urgency INTEGER NOT NULL DEFAULT 1,
                    created_at INTEGER NOT NULL,
                    is_read INTEGER NOT NULL DEFAULT 0,
                    expire_timeout INTEGER NOT NULL DEFAULT -1
                );

                CREATE INDEX IF NOT EXISTS idx_notifications_created
                ON notifications(created_at DESC);
                ",
            )
            .map_err(|e| e.to_string())?;

        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn next_id(&self) -> Result<u32, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;

        let id = connection
            .query_row(
                "SELECT COALESCE(MAX(id), 0) + 1 FROM notifications",
                [],
                |row| row.get::<_, u32>(0),
            )
            .map_err(|e| e.to_string())?;

        Ok(id.max(1))
    }

    pub fn save(&self, item: &Notification) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;

        connection
            .execute(
                "
                INSERT INTO notifications (
                    id, app_name, app_icon, summary, body,
                    urgency, created_at, is_read, expire_timeout
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(id) DO UPDATE SET
                    app_name = excluded.app_name,
                    app_icon = excluded.app_icon,
                    summary = excluded.summary,
                    body = excluded.body,
                    urgency = excluded.urgency,
                    created_at = excluded.created_at,
                    is_read = 0,
                    expire_timeout = excluded.expire_timeout
                ",
                params![
                    item.id,
                    item.app_name,
                    item.app_icon,
                    item.summary,
                    item.body,
                    item.level.as_urgency(),
                    item.created_at,
                    item.is_read as i32,
                    item.expire_timeout,
                ],
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn history(&self, limit: u32) -> Result<Vec<Notification>, String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;

        let mut statement = connection
            .prepare(
                "
                SELECT id, app_name, app_icon, summary, body,
                       urgency, created_at, is_read, expire_timeout
                FROM notifications
                ORDER BY created_at DESC, id DESC
                LIMIT ?1
                ",
            )
            .map_err(|e| e.to_string())?;

        let rows = statement
            .query_map([limit.clamp(1, 500)], |row| {
                Ok(Notification {
                    id: row.get(0)?,
                    app_name: row.get(1)?,
                    app_icon: row.get(2)?,
                    summary: row.get(3)?,
                    body: row.get(4)?,
                    level: NotificationLevel::from_urgency(row.get(5)?),
                    created_at: row.get(6)?,
                    is_read: row.get::<_, i32>(7)? != 0,
                    expire_timeout: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn mark_read(&self, id: u32) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;

        connection
            .execute(
                "UPDATE notifications SET is_read = 1 WHERE id = ?1",
                [id],
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn clear(&self) -> Result<(), String> {
        let connection = self.connection.lock().map_err(|e| e.to_string())?;

        connection
            .execute("DELETE FROM notifications", [])
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
