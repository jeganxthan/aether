use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    pub player: String,

    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,

    pub art_url: Option<String>,

    /// Microseconds
    pub duration: i64,

    /// Microseconds
    pub position: i64,

    pub playback_status: String,

    pub can_play: bool,
    pub can_pause: bool,
    pub can_next: bool,
    pub can_previous: bool,
    pub can_seek: bool,
}

impl MediaInfo {
    pub fn is_playing(&self) -> bool {
        self.playback_status == "Playing"
    }
}