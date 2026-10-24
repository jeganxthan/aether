use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use image::ImageFormat;
use reqwest::{Client, Url};
use zbus::Connection;
use zbus::fdo::DBusProxy;
use zvariant::{OwnedValue, Value};

const MAX_ARTWORK_BYTES: usize = 12 * 1024 * 1024;
const ARTWORK_RETRY_DELAY: Duration = Duration::from_secs(30);
static ARTWORK_ATTEMPTS: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerBrand {
    Brave,
    Google,
    Spotify,
    Other,
}

impl PlayerBrand {
    pub fn from_bus_name(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower.contains("brave") {
            Self::Brave
        } else if lower.contains("spotify") {
            Self::Spotify
        } else if lower.contains("chrome") || lower.contains("google") || lower.contains("chromium") {
            Self::Google
        } else {
            Self::Other
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: Option<String>,
    pub length_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerState {
    pub bus_name: String,
    pub brand: PlayerBrand,
    pub is_playing: bool,
    pub position_secs: u64,
    pub metadata: PlayerMetadata,
}

impl PlayerState {
    /// Formatted remaining time e.g. "-1:13", or elapsed time "1:23"
    pub fn time_display(&self) -> String {
        if let Some(len) = self.metadata.length_secs {
            if len >= self.position_secs {
                let remaining = len - self.position_secs;
                let mins = remaining / 60;
                let secs = remaining % 60;
                return format!("-{mins}:{secs:02}");
            }
        }
        let mins = self.position_secs / 60;
        let secs = self.position_secs % 60;
        format!("{mins}:{secs:02}")
    }

    /// Single line "Artist - Title" or "Title"
    pub fn display_title(&self) -> String {
        if !self.metadata.artist.is_empty() && !self.metadata.title.is_empty() {
            format!("{} — {}", self.metadata.artist, self.metadata.title)
        } else if !self.metadata.title.is_empty() {
            self.metadata.title.clone()
        } else if !self.metadata.artist.is_empty() {
            self.metadata.artist.clone()
        } else {
            "Playing Media".to_string()
        }
    }

    /// Local filesystem path to the album art image, if available
    pub fn local_art_path(&self) -> Option<PathBuf> {
        let url = self.metadata.art_url.as_deref()?;
        if let Some(path) = artwork_cache_path(url) {
            return path.exists().then_some(path);
        }

        let path = if let Some(path) = url.strip_prefix("file://") {
            let path = if let Some(path) = path.strip_prefix("localhost/") {
                format!("/{path}")
            } else {
                path.to_string()
            };
            let decoded = decode_file_uri_path(&path)?;
            PathBuf::from(decoded)
        } else if url.contains("://") {
            return None;
        } else {
            PathBuf::from(url)
        };

        path.exists().then_some(path)
    }
}

pub async fn cache_remote_artwork(url: &str) -> Option<PathBuf> {
    let path = artwork_cache_path(url)?;
    if path.exists() || !may_attempt_artwork(url) {
        return path.exists().then_some(path);
    }

    let result = async {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;
        let mut response = client.get(url).send().await.ok()?;
        if !response.status().is_success() {
            return None;
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_ARTWORK_BYTES as u64)
        {
            return None;
        }

        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.ok()? {
            if bytes.len() + chunk.len() > MAX_ARTWORK_BYTES {
                return None;
            }
            bytes.extend_from_slice(&chunk);
        }

        let decoded = image::load_from_memory(&bytes).ok()?;
        let mut png = Cursor::new(Vec::new());
        decoded.write_to(&mut png, ImageFormat::Png).ok()?;

        let parent = path.parent()?;
        std::fs::create_dir_all(parent).ok()?;
        let temporary = path.with_extension("png.tmp");
        std::fs::write(&temporary, png.into_inner()).ok()?;
        std::fs::rename(&temporary, &path).ok()?;

        Some(path)
    }
    .await;

    result
}

fn artwork_cache_path(url: &str) -> Option<PathBuf> {
    let url = Url::parse(url).ok()?;
    if url.scheme() != "https" || !is_artwork_host(url.host_str()?) {
        return None;
    }

    let cache_dir = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))?;

    let mut hasher = DefaultHasher::new();
    url.as_str().hash(&mut hasher);
    Some(
        cache_dir
            .join("aether")
            .join("player-art")
            .join(format!("{:016x}.png", hasher.finish())),
    )
}

fn is_artwork_host(host: &str) -> bool {
    host == "scdn.co" || host.ends_with(".scdn.co")
}

fn may_attempt_artwork(url: &str) -> bool {
    let attempts = ARTWORK_ATTEMPTS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut attempts) = attempts.lock() else {
        return false;
    };

    let now = Instant::now();
    attempts.retain(|_, last_attempt| now.duration_since(*last_attempt) < ARTWORK_RETRY_DELAY);
    if attempts
        .get(url)
        .is_some_and(|last_attempt| now.duration_since(*last_attempt) < ARTWORK_RETRY_DELAY)
    {
        return false;
    }

    attempts.insert(url.to_owned(), now);
    true
}

fn decode_file_uri_path(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = bytes.get(index + 1..index + 3)?;
            let hex = std::str::from_utf8(hex).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).ok()
}

pub struct PlayerService {
    connection: Connection,
}

impl PlayerService {
    pub async fn new() -> zbus::Result<Self> {
        let connection = Connection::session().await?;
        Ok(Self { connection })
    }

    pub async fn current_player(&self) -> Option<PlayerState> {
        let dbus = DBusProxy::new(&self.connection).await.ok()?;
        let names = dbus.list_names().await.ok()?;

        let mut candidate_players = Vec::new();
        for name in names {
            if name.starts_with("org.mpris.MediaPlayer2.") {
                candidate_players.push(name.to_string());
            }
        }

        if candidate_players.is_empty() {
            return None;
        }

        // Check each player's status
        let mut playing_player = None;
        let mut paused_player = None;

        for bus_name in candidate_players {
            let Ok(proxy) = zbus::Proxy::new(
                &self.connection,
                bus_name.as_str(),
                "/org/mpris/MediaPlayer2",
                "org.mpris.MediaPlayer2.Player",
            )
            .await else {
                continue;
            };

            let status: String = proxy.get_property("PlaybackStatus").await.unwrap_or_default();
            let is_playing = status.eq_ignore_ascii_case("Playing");
            let is_paused = status.eq_ignore_ascii_case("Paused");

            if !is_playing && !is_paused {
                continue;
            }

            let metadata_map: HashMap<String, OwnedValue> =
                proxy.get_property("Metadata").await.unwrap_or_default();

            let title = metadata_map
                .get("xesam:title")
                .and_then(|v| <&str>::try_from(v).ok())
                .unwrap_or("")
                .to_string();

            let mut artist = String::new();
            if let Some(v) = metadata_map.get("xesam:artist") {
                match &**v {
                    Value::Array(arr) => {
                        for elem in arr.iter() {
                            if let Ok(s) = <&str>::try_from(elem) {
                                if !artist.is_empty() {
                                    artist.push_str(", ");
                                }
                                artist.push_str(s);
                            }
                        }
                    }
                    Value::Str(s) => artist = s.to_string(),
                    _ => {}
                }
            }

            let album = metadata_map
                .get("xesam:album")
                .and_then(|v| <&str>::try_from(v).ok())
                .unwrap_or("")
                .to_string();

            let art_url = metadata_map
                .get("mpris:artUrl")
                .and_then(|v| <&str>::try_from(v).ok())
                .map(String::from);

            let length_us = metadata_map
                .get("mpris:length")
                .and_then(|v| <i64>::try_from(v).ok())
                .unwrap_or(0);

            let length_secs = if length_us > 0 {
                Some((length_us / 1_000_000) as u64)
            } else {
                None
            };

            let position_us = proxy.get_property::<i64>("Position").await.unwrap_or(0);
            let position_secs = if position_us > 0 {
                (position_us / 1_000_000) as u64
            } else {
                0
            };

            let brand = PlayerBrand::from_bus_name(&bus_name);

            let state = PlayerState {
                bus_name: bus_name.clone(),
                brand,
                is_playing,
                position_secs,
                metadata: PlayerMetadata {
                    title,
                    artist,
                    album,
                    art_url,
                    length_secs,
                },
            };

            if is_playing {
                playing_player = Some(state);
                break;
            } else if paused_player.is_none() {
                paused_player = Some(state);
            }
        }

        playing_player.or(paused_player)
    }

    pub async fn play_pause(&self, bus_name: &str) -> zbus::Result<()> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            bus_name,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .await?;

        let _: () = proxy.call("PlayPause", &()).await?;
        Ok(())
    }

    pub async fn next(&self, bus_name: &str) -> zbus::Result<()> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            bus_name,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .await?;

        let _: () = proxy.call("Next", &()).await?;
        Ok(())
    }

    pub async fn previous(&self, bus_name: &str) -> zbus::Result<()> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            bus_name,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .await?;

        let _: () = proxy.call("Previous", &()).await?;
        Ok(())
    }

    pub async fn seek(&self, bus_name: &str, offset_us: i64) -> zbus::Result<()> {
        let proxy = zbus::Proxy::new(
            &self.connection,
            bus_name,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .await?;

        let _: () = proxy.call("Seek", &(offset_us)).await?;
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::{decode_file_uri_path, is_artwork_host};

    #[test]
    fn decodes_percent_escaped_file_uri_paths() {
        assert_eq!(
            decode_file_uri_path("/tmp/album%20art%2Bcover.jpg").as_deref(),
            Some("/tmp/album art+cover.jpg")
        );
        assert_eq!(decode_file_uri_path("/tmp/bad%2"), None);
        assert_eq!(decode_file_uri_path("/tmp/%FF"), None);
    }

    #[test]
    fn artwork_downloads_are_limited_to_spotify_cdn_hosts() {
        assert!(is_artwork_host("i.scdn.co"));
        assert!(is_artwork_host("image.scdn.co"));
        assert!(!is_artwork_host("scdn.co.attacker.example"));
        assert!(!is_artwork_host("example.com"));
    }
}
