use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotificationLevel {
    Critical,
    Warning,
    Normal,
}

impl Default for NotificationLevel {
    fn default() -> Self {
        Self::Normal
    }
}

impl NotificationLevel {
    /// D-Bus `urgency` levels: 0 = low, 1 = normal, 2 = critical.
    pub fn as_urgency(self) -> i32 {
        match self {
            Self::Normal => 0,
            Self::Warning => 1,
            Self::Critical => 2,
        }
    }

    pub fn from_urgency(value: i32) -> Self {
        match value {
            2 => Self::Critical,
            _ => Self::Normal,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    #[serde(default)]
    pub level: NotificationLevel,
    pub created_at: i64,
    pub is_read: bool,
    pub expire_timeout: i32,
}