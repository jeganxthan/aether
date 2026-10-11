use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEntry {
    pub id: String,
    pub content: String,
    pub content_type: String,
    pub copied_at: i64,
    pub pinned: bool,
}
