
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use tauri::{AppHandle, Emitter};

use crate::models::notification::{Notification, NotificationLevel};

use super::repository::NotificationRepository;

#[derive(Clone)]
pub struct NotificationManager {
    app: AppHandle,
    repository: NotificationRepository,
    active: Arc<Mutex<HashSet<u32>>>,
    next_id: Arc<AtomicU32>,
}

impl NotificationManager {
    pub fn new(
        app: AppHandle,
        repository: NotificationRepository,
    ) -> Result<Self, String> {
        let next_id = repository.next_id()?;

        Ok(Self {
            app,
            repository,
            active: Arc::new(Mutex::new(HashSet::new())),
            next_id: Arc::new(AtomicU32::new(next_id)),
        })
    }

    pub fn publish(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        expire_timeout: i32,
        level: NotificationLevel,
    ) -> Result<u32, String> {
        let id = if replaces_id != 0 {
            replaces_id
        } else {
            self.next_id.fetch_add(1, Ordering::Relaxed).max(1)
        };

        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let notification = Notification {
            id,
            app_name,
            app_icon,
            summary,
            body,
            level,
            created_at,
            is_read: false,
            expire_timeout,
        };

        self.repository.save(&notification)?;

        self.active
            .lock()
            .map_err(|e| e.to_string())?
            .insert(id);

        self.app
            .emit("notification:new", &notification)
            .map_err(|e| e.to_string())?;

        Ok(id)
    }

    pub fn history(&self, limit: u32) -> Result<Vec<Notification>, String> {
        self.repository.history(limit)
    }

    pub fn mark_read(&self, id: u32) -> Result<(), String> {
        self.repository.mark_read(id)
    }

    pub fn clear_history(&self) -> Result<(), String> {
        self.repository.clear()?;

        self.active
            .lock()
            .map_err(|e| e.to_string())?
            .clear();

        self.app
            .emit("notification:clear", ())
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    pub fn dismiss(&self, id: u32) -> Result<bool, String> {
        let removed = self
            .active
            .lock()
            .map_err(|e| e.to_string())?
            .remove(&id);

        if removed {
            self.app
                .emit("notification:closed", id)
                .map_err(|e| e.to_string())?;
        }

        Ok(removed)
    }
}
