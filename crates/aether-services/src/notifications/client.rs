use std::collections::HashMap;

use zbus::zvariant::OwnedValue;

/*
 * The sending side of the notification service. The panel owns the
 * org.freedesktop.Notifications name, so a notification published here is
 * shown by the shell's own popup, not by some other daemon.
 */

#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &Vec<String>,
        hints: &HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
}

// how long a published popup stays on screen, in milliseconds
const TIMEOUT_MS: i32 = 5000;

/*
 * Publishes a notification and returns the id the server gave it. The body is
 * plain text: markup and entities are decoded by the popup that draws it.
 */
pub async fn publish(
    app_name: &str,
    app_icon: &str,
    summary: &str,
    body: &str,
) -> Result<u32, String> {
    publish_over(None, app_name, app_icon, summary, body).await
}

/*
 * The same, for callers that are not inside a runtime already. It runs on a
 * thread of its own, so it also works from inside one.
 */
pub fn publish_now(app_name: &str, app_icon: &str, summary: &str, body: &str) -> Result<u32, String> {
    let app_name = app_name.to_string();
    let app_icon = app_icon.to_string();
    let summary = summary.to_string();
    let body = body.to_string();

    let (sender, receiver) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| format!("Could not start a runtime: {error}"));

        let result = match runtime {
            Ok(runtime) => runtime.block_on(publish(&app_name, &app_icon, &summary, &body)),
            Err(error) => Err(error),
        };

        let _ = sender.send(result);
    });

    receiver
        .recv()
        .unwrap_or_else(|_| Err("The notification thread stopped early".to_string()))
}

// the same, over a bus of the caller's choosing, which is what the test uses
async fn publish_over(
    address: Option<String>,
    app_name: &str,
    app_icon: &str,
    summary: &str,
    body: &str,
) -> Result<u32, String> {
    let connection = match address {
        Some(address) => {
            let address = zbus::Address::try_from(address.as_str())
                .map_err(|error| format!("Could not use that bus address: {error}"))?;

            zbus::connection::Builder::address(address)
                .map_err(|error| format!("Could not use that bus address: {error}"))?
                .build()
                .await
                .map_err(|error| format!("Could not reach the session bus: {error}"))?
        }
        None => zbus::Connection::session()
            .await
            .map_err(|error| format!("Could not reach the session bus: {error}"))?,
    };

    let proxy = NotificationsProxy::new(&connection)
        .await
        .map_err(|error| format!("Could not reach the notification server: {error}"))?;

    proxy
        .notify(
            app_name,
            0,
            app_icon,
            summary,
            body,
            &Vec::new(),
            &HashMap::new(),
            TIMEOUT_MS,
        )
        .await
        .map_err(|error| format!("The notification server refused the notice: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_bus_that_is_not_there_fails_instead_of_panicking() {
        let result = publish_over(
            Some("unix:path=/nonexistent/aether-test-bus".to_string()),
            "Aether",
            "",
            "Summary",
            "Body",
        )
        .await;

        assert!(result.is_err());
    }
}
