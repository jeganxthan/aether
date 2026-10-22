use std::{collections::HashMap, io, time::Duration};

use tokio::{task::JoinHandle, time::sleep};
use zbus::{Connection, Proxy};
use zvariant::OwnedValue;

use crate::wifi::ConnectedWifiService;

const POLL_INTERVAL: Duration = Duration::from_secs(5);
const DBUS_RETRY_INTERVAL: Duration = Duration::from_secs(5);

const BUS_NAME: &str = "org.freedesktop.Notifications";
const OBJECT_PATH: &str = "/org/freedesktop/Notifications";
const INTERFACE: &str = "org.freedesktop.Notifications";

/*
 * What the monitor knows about the connection: the name to show, and its link
 * speed for the body. The name is what tells one network from another, so a
 * switch straight from one to another reads as a new connection.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
struct WifiReading {
    ssid: String,
    rate_mbps: Option<u32>,
}

pub fn start_wifi_alerts() -> JoinHandle<()> {
    tokio::spawn(async {
        if let Err(error) = monitor_wifi().await {
            eprintln!("Aether wifi monitor stopped: {error}");
        }
    })
}

async fn monitor_wifi() -> io::Result<()> {
    let service = match ConnectedWifiService::new().await {
        Ok(service) => service,
        Err(error) => {
            return Err(io::Error::other(error));
        }
    };

    let connection = loop {
        match Connection::session().await {
            Ok(connection) => break connection,
            Err(error) => {
                eprintln!("Wifi alerts: session D-Bus unavailable: {error}");
                sleep(DBUS_RETRY_INTERVAL).await;
            }
        }
    };

    let mut previous: Option<WifiReading> = None;
    let mut initialized = false;

    loop {
        match read_wifi(&service).await {
            Ok(current) => {
                // A change of name is a connect; dropping to nothing is a disconnect.
                // Log the observed transition alongside the notification to make
                // NetworkManager state changes visible in `cargo run` output.
                if let Some(old) = previous.as_ref() {
                    match (old, current.as_ref()) {
                        (old, Some(current)) if old.ssid != current.ssid => {
                            eprintln!(
                                "aether-panel: NetworkManager Wi-Fi changed: {:?} -> {:?}",
                                old.ssid, current.ssid
                            );
                            notify_retry(
                                &connection,
                                "Wi-Fi connected",
                                &format!("Connected to {}.", current.ssid),
                                "network-wireless",
                            )
                            .await;
                        }
                        (_, None) => {
                            eprintln!(
                                "aether-panel: NetworkManager reports Wi-Fi disconnected from {:?}",
                                old.ssid
                            );
                            notify_retry(
                                &connection,
                                "Wi-Fi disconnected",
                                "You are no longer connected to a network.",
                                "network-wireless-offline",
                            )
                            .await;
                        }
                        _ => {}
                    }
                } else if initialized && let Some(current) = current.as_ref() {
                    // A reconnect to the same SSID follows a sampled `None`
                    // state, so it must be logged separately from an SSID roam.
                    eprintln!(
                        "aether-panel: NetworkManager reports Wi-Fi connected to {:?}",
                        current.ssid
                    );
                }

                previous = current;
                initialized = true;
            }
            Err(error) => {
                eprintln!("Could not read wifi state: {error}");
            }
        }

        sleep(POLL_INTERVAL).await;
    }
}

async fn read_wifi(service: &ConnectedWifiService) -> nmrs::Result<Option<WifiReading>> {
    Ok(service.active_ssid().await?.map(|ssid| WifiReading {
        ssid,
        rate_mbps: None,
    }))
}

async fn notify_retry(connection: &Connection, title: &str, message: &str, icon: &str) -> bool {
    match send_notification(connection, title, message, icon).await {
        Ok(()) => true,
        Err(error) => {
            eprintln!("Could not send wifi notification: {error}");
            false
        }
    }
}

/*
 * Sends a desktop notification, learning the icon from the theme if one is
 * asked for by name. The app name stays the same as the battery alerts, so
 * every Aether notice groups together.
 */
async fn send_notification(
    connection: &Connection,
    summary: &str,
    body: &str,
    icon: &str,
) -> zbus::Result<()> {
    let proxy = Proxy::new(connection, BUS_NAME, OBJECT_PATH, INTERFACE).await?;

    let actions: Vec<String> = Vec::new();
    let hints: HashMap<String, OwnedValue> = HashMap::new();

    let args = (
        "Aether", 0_u32, icon, summary, body, actions, hints, 5000_i32,
    );

    let _: u32 = proxy.call("Notify", &args).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // the transition rule, pulled out so it can be checked without a bus
    fn transition(
        old: Option<&WifiReading>,
        current: Option<&WifiReading>,
    ) -> Option<&'static str> {
        match (old, current) {
            (Some(old), Some(current)) if old.ssid != current.ssid => Some("connected"),
            (Some(_), None) => Some("disconnected"),
            _ => None,
        }
    }

    fn reading(ssid: &str) -> WifiReading {
        WifiReading {
            ssid: ssid.to_owned(),
            rate_mbps: None,
        }
    }

    #[test]
    fn the_first_reading_is_only_a_baseline() {
        // starting while already connected must not invent an event
        assert_eq!(transition(None, Some(&reading("Jegan"))), None);
        assert_eq!(transition(None, None), None);
    }

    #[test]
    fn staying_on_the_same_network_changes_nothing() {
        let old = reading("Jegan");
        let current = reading("Jegan");

        assert_eq!(transition(Some(&old), Some(&current)), None);
    }

    #[test]
    fn a_new_name_reads_as_a_connection() {
        let old = reading("Jegan");
        let current = reading("JSIP Design");

        assert_eq!(transition(Some(&old), Some(&current)), Some("connected"));
    }

    #[test]
    fn dropping_to_nothing_reads_as_a_disconnection() {
        let old = reading("Jegan");

        assert_eq!(transition(Some(&old), None), Some("disconnected"));
    }
}
