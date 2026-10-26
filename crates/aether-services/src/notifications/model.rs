use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub category: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<String>,
    pub created_at: u64,
    pub expire_timeout_ms: i32,
    pub closed: bool,
}

impl Notification {
    pub fn new(
        id: u32,
        app_name: String,
        app_icon: String,
        category: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        expire_timeout_ms: i32,
    ) -> Self {
        Self {
            id,
            app_name,
            app_icon,
            category,
            summary,
            body,
            actions,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            expire_timeout_ms,
            closed: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum NotificationEvent {
    Created(Notification),
    Closed { id: u32, reason: u32 },
    ActionInvoked { id: u32, action_key: String },
}
