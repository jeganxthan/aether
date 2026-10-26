use serde::{Deserialize, Serialize};
use zbus::{Connection, Proxy, fdo::ObjectManagerProxy};

const BLUEZ: &str = "org.bluez";
const OBJECT_MANAGER_PATH: &str = "/";
const ADAPTER_INTERFACE: &str = "org.bluez.Adapter1";
const DEVICE_INTERFACE: &str = "org.bluez.Device1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BluetoothDevice {
    pub path: String,
    pub address: String,
    pub name: String,
    pub paired: bool,
    pub connected: bool,
    pub trusted: bool,
    pub rssi: Option<i16>,
}

/// Holds the D-Bus connection for the entire discovery session.
/// Do not create a new Connection for stop_discovery().
pub struct DiscoverySession {
    connection: Connection,
    pub adapter_path: String,
}

async fn get_property<T>(
    connection: &Connection,
    path: &str,
    interface: &str,
    property: &str,
) -> zbus::Result<T>
where
    T: TryFrom<zbus::zvariant::OwnedValue>,
    <T as TryFrom<zbus::zvariant::OwnedValue>>::Error: Into<zbus::Error>,
{
    let proxy = Proxy::new(connection, BLUEZ, path, interface).await?;
    proxy.get_property(property).await
}

async fn get_objects(
    connection: &Connection,
) -> zbus::Result<
    std::collections::HashMap<
        zbus::zvariant::OwnedObjectPath,
        std::collections::HashMap<
            zbus::names::OwnedInterfaceName,
            std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        >,
    >,
> {
    let manager = ObjectManagerProxy::builder(connection)
        .destination(BLUEZ)?
        .path(OBJECT_MANAGER_PATH)?
        .build()
        .await?;

    manager
        .get_managed_objects()
        .await
        .map_err(zbus::Error::from)
}

pub async fn list_adapters() -> zbus::Result<Vec<String>> {
    let connection = Connection::system().await?;
    let objects = get_objects(&connection).await?;

    let mut adapters: Vec<String> = objects
        .into_iter()
        .filter_map(|(path, interfaces)| {
            interfaces
                .contains_key(ADAPTER_INTERFACE)
                .then(|| path.to_string())
        })
        .collect();

    adapters.sort();
    Ok(adapters)
}

pub async fn default_adapter() -> zbus::Result<String> {
    list_adapters()
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| zbus::Error::Failure("No Bluetooth adapter found".into()))
}

impl DiscoverySession {
    /// Start discovery and keep this D-Bus connection alive.
    pub async fn start() -> zbus::Result<Self> {
        let connection = Connection::system().await?;
        let objects = get_objects(&connection).await?;

        let mut adapters: Vec<String> = objects
            .into_iter()
            .filter_map(|(path, interfaces)| {
                interfaces
                    .contains_key(ADAPTER_INTERFACE)
                    .then(|| path.to_string())
            })
            .collect();

        adapters.sort();

        let adapter_path = adapters
            .into_iter()
            .next()
            .ok_or_else(|| zbus::Error::Failure("No Bluetooth adapter found".into()))?;

        let proxy =
            Proxy::new(&connection, BLUEZ, adapter_path.as_str(), ADAPTER_INTERFACE).await?;

        proxy.call_method("StartDiscovery", &()).await?;

        Ok(Self {
            connection,
            adapter_path,
        })
    }

    /// List devices using the same connection as StartDiscovery.
    pub async fn list_devices(&self) -> zbus::Result<Vec<BluetoothDevice>> {
        list_devices_on(&self.connection).await
    }

    /// Stop this session using the same connection.
    pub async fn stop(self) -> zbus::Result<()> {
        let proxy = Proxy::new(
            &self.connection,
            BLUEZ,
            self.adapter_path.as_str(),
            ADAPTER_INTERFACE,
        )
        .await?;

        proxy.call_method("StopDiscovery", &()).await?;
        Ok(())
    }
}

async fn list_devices_on(connection: &Connection) -> zbus::Result<Vec<BluetoothDevice>> {
    let objects = get_objects(connection).await?;
    let mut devices = Vec::new();

    for (object_path, interfaces) in objects {
        if !interfaces.contains_key(DEVICE_INTERFACE) {
            continue;
        }

        let path = object_path.to_string();

        let address: String =
            match get_property(connection, &path, DEVICE_INTERFACE, "Address").await {
                Ok(value) => value,
                Err(_) => continue,
            };

        let name: String = get_property(connection, &path, DEVICE_INTERFACE, "Name")
            .await
            .unwrap_or_else(|_| address.clone());

        let paired: bool = get_property(connection, &path, DEVICE_INTERFACE, "Paired")
            .await
            .unwrap_or(false);

        let connected: bool = get_property(connection, &path, DEVICE_INTERFACE, "Connected")
            .await
            .unwrap_or(false);

        let trusted: bool = get_property(connection, &path, DEVICE_INTERFACE, "Trusted")
            .await
            .unwrap_or(false);

        let rssi: Option<i16> = get_property(connection, &path, DEVICE_INTERFACE, "RSSI")
            .await
            .ok();

        devices.push(BluetoothDevice {
            path,
            address,
            name,
            paired,
            connected,
            trusted,
            rssi,
        });
    }

    devices.sort_by(|a, b| {
        b.connected
            .cmp(&a.connected)
            .then_with(|| b.paired.cmp(&a.paired))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(devices)
}

/// List devices already known to BlueZ without starting discovery.
pub async fn list_devices() -> zbus::Result<Vec<BluetoothDevice>> {
    let connection = Connection::system().await?;
    list_devices_on(&connection).await
}

/// Connect a device using BlueZ's supported connection profiles.
pub async fn connect_device(device_path: &str) -> zbus::Result<()> {
    let connection = Connection::system().await?;

    let proxy = Proxy::new(&connection, BLUEZ, device_path, DEVICE_INTERFACE).await?;

    proxy.call_method("Connect", &()).await?;

    Ok(())
}

/// Disconnect a connected device.
pub async fn disconnect_device(device_path: &str) -> zbus::Result<()> {
    let connection = Connection::system().await?;

    let proxy = Proxy::new(&connection, BLUEZ, device_path, DEVICE_INTERFACE).await?;

    proxy.call_method("Disconnect", &()).await?;

    Ok(())
}

/// Pair a device. The BlueZ pairing agent may request confirmation or a PIN.
pub async fn pair_device(device_path: &str) -> zbus::Result<()> {
    let connection = Connection::system().await?;

    let proxy = Proxy::new(&connection, BLUEZ, device_path, DEVICE_INTERFACE).await?;

    proxy.call_method("Pair", &()).await?;

    Ok(())
}
