use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use tokio::sync::broadcast;
use zbus::{Connection, interface, object_server::SignalEmitter};
use zvariant::OwnedValue;

use super::model::{Notification, NotificationEvent};

const BUS_NAME: &str = "org.freedesktop.Notifications";
const OBJECT_PATH: &str = "/org/freedesktop/Notifications";
const MAX_HISTORY: usize = 100;

#[derive(Default)]
struct Store {
    next_id: u32,
    history: VecDeque<Notification>,
}

impl Store {
    fn allocate_id(&mut self) -> u32 {
        loop {
            self.next_id = self.next_id.wrapping_add(1).max(1);

            if !self.history.iter().any(|item| item.id == self.next_id) {
                return self.next_id;
            }
        }
    }

    fn push(&mut self, notification: Notification) {
        self.history.push_back(notification);

        while self.history.len() > MAX_HISTORY {
            self.history.pop_front();
        }
    }
}

struct NotificationInterface {
    store: Arc<Mutex<Store>>,
    events: broadcast::Sender<NotificationEvent>,
}

#[interface(name = "org.freedesktop.Notifications")]
impl NotificationInterface {
    async fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: std::collections::HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let category = hints
            .get("category")
            .and_then(|value| String::try_from(value.clone()).ok())
            .unwrap_or_default();
        let mut store = self.store.lock().expect("notification store poisoned");

        let id = if replaces_id != 0 && store.history.iter().any(|item| item.id == replaces_id) {
            replaces_id
        } else {
            store.allocate_id()
        };

        // Replace the existing history entry when a valid ID is supplied.
        if let Some(existing) = store.history.iter_mut().find(|item| item.id == id) {
            existing.app_name = app_name.clone();
            existing.app_icon = app_icon.clone();
            existing.category = category.clone();
            existing.summary = summary.clone();
            existing.body = body.clone();
            existing.actions = actions.clone();
            existing.created_at = super::model::Notification::new(
                id,
                app_name.clone(),
                app_icon.clone(),
                category.clone(),
                summary.clone(),
                body.clone(),
                actions.clone(),
                expire_timeout,
            )
            .created_at;
            existing.expire_timeout_ms = expire_timeout;
            existing.closed = false;

            let notification = existing.clone();
            drop(store);
            let _ = self.events.send(NotificationEvent::Created(notification));
            return id;
        }

        let notification = Notification::new(
            id,
            app_name,
            app_icon,
            category,
            summary,
            body,
            actions,
            expire_timeout,
        );

        store.push(notification.clone());
        drop(store);

        let _ = self.events.send(NotificationEvent::Created(notification));
        id
    }

    async fn close_notification(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        id: u32,
    ) -> zbus::fdo::Result<()> {
        let found = {
            let mut store = self.store.lock().expect("notification store poisoned");

            if let Some(item) = store.history.iter_mut().find(|item| item.id == id) {
                item.closed = true;
                true
            } else {
                false
            }
        };

        if found {
            let _ = self
                .events
                .send(NotificationEvent::Closed { id, reason: 3 });
            Self::notification_closed(&emitter, id, 3).await?;
        }

        Ok(())
    }

    async fn get_capabilities(&self) -> Vec<String> {
        vec![
            "body".into(),
            "body-markup".into(),
            "actions".into(),
            "persistence".into(),
        ]
    }

    async fn get_server_information(&self) -> (String, String, String, String) {
        (
            "Aether Shell".into(),
            "Aether Project".into(),
            env!("CARGO_PKG_VERSION").into(),
            "1.2".into(),
        )
    }

    #[zbus(signal)]
    async fn notification_closed(
        emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn action_invoked(
        emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;
}

#[derive(Clone)]
pub struct NotificationService {
    connection: Connection,
    store: Arc<Mutex<Store>>,
    events: broadcast::Sender<NotificationEvent>,
}

impl NotificationService {
    pub async fn start() -> zbus::Result<Self> {
        let connection = Connection::session().await?;
        let store = Arc::new(Mutex::new(Store::default()));
        let (events, _) = broadcast::channel(128);

        let interface = NotificationInterface {
            store: store.clone(),
            events: events.clone(),
        };

        connection
            .object_server()
            .at(OBJECT_PATH, interface)
            .await?;
        connection.request_name(BUS_NAME).await?;

        Ok(Self {
            connection,
            store,
            events,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<NotificationEvent> {
        self.events.subscribe()
    }

    pub fn history(&self) -> Vec<Notification> {
        self.store
            .lock()
            .expect("notification store poisoned")
            .history
            .iter()
            .cloned()
            .collect()
    }

    pub fn active(&self) -> Vec<Notification> {
        self.store
            .lock()
            .expect("notification store poisoned")
            .history
            .iter()
            .filter(|item| !item.closed)
            .cloned()
            .collect()
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires a private D-Bus session"]
    async fn service_accepts_and_closes_notifications() {
        let service = NotificationService::start()
            .await
            .expect("notification server should register its D-Bus name");
        let id = super::super::client::publish("test", "", "hello", "world")
            .await
            .expect("notification request should reach the server");

        assert_eq!(service.active().len(), 1);

        super::super::client::close_notification(id)
            .await
            .expect("notification close request should reach the server");
        assert!(service.active().is_empty());
    }
}
