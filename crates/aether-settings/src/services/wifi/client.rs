use std::{future::Future, io, time::Duration};

use zbus::{Connection, Proxy};

const SERVICE_NAME: &str = "org.aether.Service";
const OBJECT_PATH: &str = "/org/aether/Network/Wifi";
const INTERFACE: &str = "org.aether.Wifi";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const SCAN_TIMEOUT: Duration = Duration::from_secs(30);
const SCAN_REFRESH_ATTEMPTS: usize = 6;
const SCAN_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

type ServiceResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct WifiNetwork {
    pub ssid: String,
    pub strength: u8,
    pub is_saved: bool,
}

#[derive(Debug, Clone)]
pub struct ConnectedNetwork {
    pub ssid: String,
    pub interface: String,
    pub bssid: String,
    pub strength: u8,
    pub ip4_address: String,
    pub ip6_address: String,
    pub password: String,
    pub qr_payload: String,
}

#[derive(Clone)]
pub struct WifiClient {
    connection: Connection,
}

use futures::future::{Either, select};
use futures::pin_mut;
use futures_timer::Delay;

async fn with_timeout<T, F>(duration: Duration, future: F) -> ServiceResult<T>
where
    F: Future<Output = ServiceResult<T>>,
{
    let delay = Delay::new(duration);

    pin_mut!(future, delay);

    match select(future, delay).await {
        Either::Left((result, _)) => result,
        Either::Right((_, _)) => {
            Err(io::Error::new(io::ErrorKind::TimedOut, "Wi-Fi service request timed out").into())
        }
    }
}
impl WifiClient {
    pub async fn connect() -> ServiceResult<Self> {
        let connection = with_timeout(CONNECT_TIMEOUT, async {
            let connection = Connection::session().await?;
            Ok(connection)
        })
        .await?;

        Ok(Self { connection })
    }

    pub async fn is_enabled(&self) -> ServiceResult<bool> {
        let proxy = self.proxy().await?;

        with_timeout(REQUEST_TIMEOUT, async {
            let enabled: bool = proxy.call("IsWifiEnabled", &()).await?;
            Ok(enabled)
        })
        .await
    }

    pub async fn set_enabled(&self, enabled: bool) -> ServiceResult<()> {
        let proxy = self.proxy().await?;

        with_timeout(REQUEST_TIMEOUT, async {
            proxy
                .call::<_, _, ()>("SetWifiEnabled", &(enabled,))
                .await?;

            Ok(())
        })
        .await
    }

    pub async fn scan_networks(&self) -> ServiceResult<()> {
        let proxy = self.proxy().await?;

        with_timeout(SCAN_TIMEOUT, async {
            let _: () = proxy.call("ScanNetworks", &()).await?;
            Ok(())
        })
        .await
    }

    pub async fn list_networks(&self) -> ServiceResult<Vec<WifiNetwork>> {
        let proxy = self.proxy().await?;

        let networks: Vec<(String, u8, bool)> = with_timeout(REQUEST_TIMEOUT, async {
            let networks: Vec<(String, u8, bool)> = proxy.call("ListNetworks", &()).await?;

            Ok(networks)
        })
        .await?;

        Ok(networks
            .into_iter()
            .map(|(ssid, strength, is_saved)| WifiNetwork {
                ssid,
                strength,
                is_saved,
            })
            .collect())
    }

    pub async fn scan_and_list_networks(&self) -> ServiceResult<Vec<WifiNetwork>> {
        let mut networks = Vec::new();

        for attempt in 0..SCAN_REFRESH_ATTEMPTS {
            self.scan_networks().await?;
            futures_timer::Delay::new(SCAN_REFRESH_INTERVAL).await;
            networks = self.list_networks().await?;

            if !networks.is_empty() || attempt + 1 == SCAN_REFRESH_ATTEMPTS {
                break;
            }
        }

        Ok(networks)
    }

    pub async fn connected_wifi(&self) -> ServiceResult<Option<ConnectedNetwork>> {
        let proxy = self.proxy().await?;

        let networks: Vec<(String, String, String, u8, String, String, String, String)> =
            with_timeout(REQUEST_TIMEOUT, async {
                let networks: Vec<_> = proxy.call("ConnectedWifi", &()).await?;

                Ok(networks)
            })
            .await?;

        let Some((
            ssid,
            interface,
            bssid,
            strength,
            ip4_address,
            ip6_address,
            password,
            qr_payload,
        )) = networks.into_iter().next()
        else {
            return Ok(None);
        };

        if ssid.is_empty() {
            return Ok(None);
        }

        Ok(Some(ConnectedNetwork {
            ssid,
            interface,
            bssid,
            strength,
            ip4_address,
            ip6_address,
            password,
            qr_payload,
        }))
    }

    pub async fn connect_network(
        &self,
        ssid: &str,
        password: Option<&str>,
        interface: Option<&str>,
    ) -> ServiceResult<()> {
        let proxy = self.proxy().await?;

        with_timeout(REQUEST_TIMEOUT, async {
            let _: () = proxy
                .call(
                    "ConnectNetwork",
                    &(ssid, password.unwrap_or(""), interface.unwrap_or("")),
                )
                .await?;

            Ok(())
        })
        .await
    }

    async fn proxy<'a>(&'a self) -> ServiceResult<Proxy<'a>> {
        let proxy = Proxy::new(&self.connection, SERVICE_NAME, OBJECT_PATH, INTERFACE).await?;

        Ok(proxy)
    }
}
