
use std::collections::HashMap;

use zbus::{
    connection,
    interface,
    object_server::SignalEmitter,
    zvariant::OwnedValue,
};

use super::manager::NotificationManager;
use crate::models::notification::NotificationLevel;

fn map_urgency(urgency: u8) -> NotificationLevel {
    // D-Bus urgency is 0 = low, 1 = normal, 2 = critical.
    NotificationLevel::from_urgency(urgency as i32)
}
const SERVICE_NAME: &str = "org.freedesktop.Notifications";
const OBJECT_PATH: &str = "/org/freedesktop/Notifications";

pub struct NotificationDbus {
    manager: NotificationManager,
}

impl NotificationDbus {
    pub fn new(manager: NotificationManager) -> Self {
        Self { manager }
    }
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationDbus {
    fn get_capabilities(&self) -> Vec<String> {
        vec!["body".to_string()]
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "Aether Notifications".into(),
            "Aether DE".into(),
            env!("CARGO_PKG_VERSION").into(),
            "1.2".into(),
        )
    }

    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> zbus::fdo::Result<u32> {
        let _ = actions;

        // Read the standard urgency hint when present.
        let urgency = hints
            .get("urgency")
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(1)
            .min(2);

        self.manager
            .publish(
                app_name,
                replaces_id,
                app_icon,
                summary,
                body,
                expire_timeout,
                map_urgency(urgency),
            )
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn close_notification(
        &self,
        id: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let removed = self
            .manager
            .dismiss(id)
            .map_err(zbus::fdo::Error::Failed)?;

        if !removed {
            return Err(zbus::fdo::Error::Failed(
                format!("Notification {id} is not active"),
            ));
        }

        Self::notification_closed(&emitter, id, 3)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        Ok(())
    }

    #[zbus(signal)]
    async fn notification_closed(
        emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;
}

pub async fn run(manager: NotificationManager) -> Result<(), String> {
    let _connection = connection::Builder::session()
        .map_err(|e| e.to_string())?
        .name(SERVICE_NAME)
        .map_err(|e| e.to_string())?
        .serve_at(OBJECT_PATH, NotificationDbus::new(manager))
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;

    // Keep the connection and its service alive for the session.
    std::future::pending::<()>().await;

    #[allow(unreachable_code)]
    Ok(())
}
