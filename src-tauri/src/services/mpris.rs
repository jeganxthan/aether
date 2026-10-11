use std::{
    collections::HashMap,
    sync::Arc,
};

use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

use zbus::{
    fdo::DBusProxy,
    zvariant::OwnedValue,
    Connection,
    Proxy,
};

use crate::models::media::MediaInfo;

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER_INTERFACE: &str = "org.mpris.MediaPlayer2.Player";


#[derive(Clone)]
pub struct MediaService {
    connection: Connection,

    /// Latest state of every discovered MPRIS player.
    players: Arc<RwLock<HashMap<String, MediaInfo>>>,

    /// Currently selected player.
    active_player: Arc<RwLock<Option<String>>>,

    app: AppHandle,
}


impl MediaService {
    pub async fn new(app: AppHandle) -> Result<Arc<Self>, zbus::Error> {
        let connection = Connection::session().await?;

        Ok(Arc::new(Self {
            connection,
            players: Arc::new(RwLock::new(HashMap::new())),
            active_player: Arc::new(RwLock::new(None)),
            app,
        }))
    }


    pub async fn start(self: Arc<Self>) {
        let service = self.clone();

        tauri::async_runtime::spawn(async move {
            if let Err(err) = service.run().await {
                eprintln!("[Aether Media] listener stopped: {err}");
            }
        });

        /*
         * Second-by-second position ticker.
         *
         * Spotify does not emit Position PropertiesChanged,
         * only PlaybackStatus/Metadata/Rate, and WebKit throttles
         * rAF/timers in this composited always-on-top window, so
         * neither event-driven Rust nor frontend timers can keep
         * the progress bar running. While the active player is
         * playing, refresh it once a second and push the fresh
         * position to the frontend.
         */
        let ticker = self.clone();

        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(1),
            );

            interval.set_missed_tick_behavior(
                tokio::time::MissedTickBehavior::Skip,
            );

            loop {
                interval.tick().await;

                ticker.refresh_active().await;
            }
        });
    }


    async fn run(self: Arc<Self>) -> Result<(), zbus::Error> {
        println!("[Aether Media] connecting to session D-Bus");

        let dbus = DBusProxy::new(&self.connection).await?;

        /*
         * Discover players which already exist.
         */
        let names = dbus.list_names().await?;

        for name in names {
            let name = name.to_string();

            if Self::is_mpris_player(&name) {
                self.add_player(name).await;
            }
        }

        /*
         * Listen for players appearing/disappearing.
         */
        let mut owner_stream = dbus.receive_name_owner_changed().await?;

        println!("[Aether Media] listening for MPRIS players");

        while let Some(signal) = owner_stream.next().await {
            let args = match signal.args() {
                Ok(args) => args,
                Err(err) => {
                    eprintln!(
                        "[Aether Media] invalid NameOwnerChanged: {err}"
                    );

                    continue;
                }
            };

            let name = args.name().to_string();

            if !Self::is_mpris_player(&name) {
                continue;
            }

            let has_owner = args.new_owner().is_some();

            if has_owner {
                println!(
                    "[Aether Media] player appeared: {name}"
                );

                self.add_player(name).await;
            } else {
                println!(
                    "[Aether Media] player disappeared: {name}"
                );

                self.remove_player(&name).await;
            }
        }

        Ok(())
    }


    fn is_mpris_player(name: &str) -> bool {
        name.starts_with(MPRIS_PREFIX)
    }


    async fn add_player(&self, name: String) {
        let service = self.clone();

        /*
         * Read the initial player state immediately.
         */
        match service.read_player(&name).await {
            Ok(media) => {
                service.update_player(media).await;
            }

            Err(err) => {
                eprintln!(
                    "[Aether Media] failed reading {name}: {err}"
                );

                return;
            }
        }


        /*
         * Start listening for PropertiesChanged.
         */
        tauri::async_runtime::spawn(async move {
            if let Err(err) = service.watch_player(name.clone()).await {
                eprintln!(
                    "[Aether Media] watcher for {name} stopped: {err}"
                );
            }
        });
    }


    async fn remove_player(&self, name: &str) {
        {
            let mut players = self.players.write().await;
            players.remove(name);
        }

        let should_clear = {
            let active = self.active_player.read().await;

            active
                .as_deref()
                .map(|active| active == name)
                .unwrap_or(false)
        };

        if should_clear {
            let mut active = self.active_player.write().await;
            *active = None;

            self.emit_media(None);
        }
    }


    async fn watch_player(
        &self,
        name: String,
    ) -> Result<(), zbus::Error> {
        let properties = zbus::fdo::PropertiesProxy::new(
            &self.connection,
            name.as_str(),
            MPRIS_PATH,
        )
        .await?;

        let mut stream =
            properties.receive_properties_changed().await?;

        while let Some(signal) = stream.next().await {
            let args = match signal.args() {
                Ok(args) => args,
                Err(err) => {
                    eprintln!(
                        "[Aether Media] PropertiesChanged error: {err}"
                    );

                    continue;
                }
            };

            /*
             * PropertiesChanged is shared by many interfaces.
             *
             * We only care about:
             *
             * org.mpris.MediaPlayer2.Player
             */
            if args.interface_name.as_str() != PLAYER_INTERFACE {
                continue;
            }

            /*
             * Instead of manually reconstructing the whole
             * metadata object from the changed fields, read the
             * current MPRIS state.
             *
             * This keeps the state consistent when several
             * properties change together.
             */
            match self.read_player(&name).await {
                Ok(media) => {
                    self.update_player(media).await;
                }

                Err(err) => {
                    eprintln!(
                        "[Aether Media] failed refreshing {name}: {err}"
                    );
                }
            }
        }

        Ok(())
    }


    async fn read_player(
        &self,
        name: &str,
    ) -> Result<MediaInfo, zbus::Error> {
        let proxy = Proxy::new(
            &self.connection,
            name,
            MPRIS_PATH,
            PLAYER_INTERFACE,
        )
        .await?;

        let playback_status: String =
            proxy.get_property("PlaybackStatus").await?;

        let metadata: HashMap<String, OwnedValue> =
            proxy.get_property("Metadata").await?;

        let position: i64 =
            proxy.get_property("Position").await.unwrap_or(0);

        let can_play: bool =
            proxy.get_property("CanPlay").await.unwrap_or(false);

        let can_pause: bool =
            proxy.get_property("CanPause").await.unwrap_or(false);

        let can_next: bool =
            proxy.get_property("CanGoNext").await.unwrap_or(false);

        let can_previous: bool =
            proxy
                .get_property("CanGoPrevious")
                .await
                .unwrap_or(false);

        let can_seek: bool =
            proxy.get_property("CanSeek").await.unwrap_or(false);


        let title = string_value(
            metadata.get("xesam:title"),
        );

        let album = string_value(
            metadata.get("xesam:album"),
        );

        let artist = artists_value(
            metadata.get("xesam:artist"),
        );

        let art_url = string_value(
            metadata.get("mpris:artUrl"),
        );

        let duration = int_value(
            metadata.get("mpris:length"),
        )
        .unwrap_or(0);


        /*
         * org.mpris.MediaPlayer2.spotify
         *
         * becomes:
         *
         * spotify
         */
        let player =
            name.strip_prefix(MPRIS_PREFIX)
                .unwrap_or(name)
                .to_string();


        Ok(MediaInfo {
            player,

            title,
            artist,
            album,

            art_url,

            duration,
            position,

            playback_status,

            can_play,
            can_pause,
            can_next,
            can_previous,
            can_seek,
        })
    }


    async fn update_player(&self, media: MediaInfo) {
        let player_name =
            format!("{MPRIS_PREFIX}{}", media.player);

        {
            let mut players = self.players.write().await;

            players.insert(
                player_name.clone(),
                media.clone(),
            );
        }


        /*
         * Prefer a player which is actually playing.
         */
        if media.is_playing() {
            let mut active =
                self.active_player.write().await;

            *active = Some(player_name);

            self.emit_media(Some(media));

            return;
        }


        /*
         * If the current player is paused, keep it available.
         *
         * The React side can decide not to display the full
         * player when playback_status != Playing.
         */
        let (adopt, emit) = {
            let active =
                self.active_player.read().await;

            match active.as_deref() {
                None => (true, true),
                Some(current) => (false, current == player_name),
            }
        };

        if adopt {
            let mut active =
                self.active_player.write().await;

            *active = Some(player_name);
        }

        if emit {
            self.emit_media(Some(media));
        }
    }


    /*
     * Ticker callback: read the active player's current state
     * every second while it is playing and re-emit it, keeping
     * the progress bar live without relying on frontend timers.
     */
    async fn refresh_active(&self) {
        let active = {
            let active =
                self.active_player.read().await;

            active.clone()
        };

        let Some(player) = active else {
            return;
        };

        let playing = {
            let players = self.players.read().await;

            players
                .get(&player)
                .map(|media| media.is_playing())
                .unwrap_or(false)
        };

        if !playing {
            return;
        }

        match self.read_player(&player).await {
            Ok(media) => {
                self.update_player(media).await;
            }

            Err(err) => {
                eprintln!(
                    "[Aether Media] ticker refresh failed: {err}"
                );
            }
        }
    }


    fn emit_media(&self, media: Option<MediaInfo>) {
        if let Err(err) =
            self.app.emit("media:update", media)
        {
            eprintln!(
                "[Aether Media] failed emitting media:update: {err}"
            );
        }
    }


    async fn active_proxy(
        &self,
    ) -> Result<Proxy<'_>, String> {
        let active = {
            let active =
                self.active_player.read().await;

            active.clone()
        };

        let Some(player) = active else {
            return Err(
                "No active MPRIS player".to_string()
            );
        };

        Proxy::new(
            &self.connection,
            player,
            MPRIS_PATH,
            PLAYER_INTERFACE,
        )
        .await
        .map_err(|err| err.to_string())
    }


    pub async fn play_pause(
        &self,
    ) -> Result<(), String> {
        let proxy = self.active_proxy().await?;

        proxy
            .call_method("PlayPause", &())
            .await
            .map(|_| ())
            .map_err(|err| err.to_string())
    }


    pub async fn next(
        &self,
    ) -> Result<(), String> {
        let proxy = self.active_proxy().await?;

        proxy
            .call_method("Next", &())
            .await
            .map(|_| ())
            .map_err(|err| err.to_string())
    }


    pub async fn previous(
        &self,
    ) -> Result<(), String> {
        let proxy = self.active_proxy().await?;

        proxy
            .call_method("Previous", &())
            .await
            .map(|_| ())
            .map_err(|err| err.to_string())
    }


    pub async fn seek(
        &self,
        offset: i64,
    ) -> Result<(), String> {
        let proxy = self.active_proxy().await?;

        proxy
            .call_method("Seek", &(offset,))
            .await
            .map(|_| ())
            .map_err(|err| err.to_string())
    }

    /*
     * Best-known state of the active player.
     *
     * The frontend bootstraps from this on mount: the MPRIS discovery emit
     * can fire before the webview's `media:update` listener exists, so the
     * initial state is read on demand instead of being lost forever.
     */
    pub async fn status(&self) -> Option<MediaInfo> {
        let active = self.active_player.read().await;

        let player = active.as_deref()?;

        let players = self.players.read().await;

        players.get(player).cloned()
    }
}


/*
 * MPRIS metadata helpers
 */

fn string_value(
    value: Option<&OwnedValue>,
) -> Option<String> {
    value
        .and_then(|value| {
            String::try_from(value.clone()).ok()
        })
}


fn artists_value(
    value: Option<&OwnedValue>,
) -> Option<String> {
    let artists =
        value.and_then(|value| {
            Vec::<String>::try_from(value.clone()).ok()
        })?;

    if artists.is_empty() {
        None
    } else {
        Some(artists.join(", "))
    }
}


fn int_value(
    value: Option<&OwnedValue>,
) -> Option<i64> {
    let value = value?;

    u64::try_from(value)
        .ok()
        .and_then(|n| i64::try_from(n).ok())
        .or_else(|| i64::try_from(value).ok())
}

#[cfg(test)]
mod tests {
    use super::int_value;
    use zbus::zvariant::OwnedValue;

    #[test]
    fn int_value_reads_unsigned_metadata() {
        let unsigned = OwnedValue::from(246_271_000u64);
        assert_eq!(int_value(Some(&unsigned)), Some(246_271_000));

        let signed = OwnedValue::from(1_234i64);
        assert_eq!(int_value(Some(&signed)), Some(1_234));

        assert_eq!(int_value(None), None);
    }
}
