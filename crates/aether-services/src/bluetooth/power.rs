use zbus::{Connection, Proxy};

const BLUEZ_SERVICE: &str = "org.bluez";
const ADAPTER_PATH: &str = "/org/bluez/hci0";
const ADAPTER_INTERFACE: &str = "org.bluez.Adapter1";

pub async fn is_powered() -> zbus::Result<bool> {
    let connection = Connection::system().await?;

    let adapter = Proxy::new(&connection, BLUEZ_SERVICE, ADAPTER_PATH, ADAPTER_INTERFACE).await?;

    adapter.get_property("Powered").await
}

pub async fn set_powered(enabled: bool) -> zbus::Result<()> {
    let connection = Connection::system().await?;

    let adapter = Proxy::new(&connection, BLUEZ_SERVICE, ADAPTER_PATH, ADAPTER_INTERFACE).await?;

    adapter.set_property("Powered", enabled).await?;

    Ok(())
}

pub async fn toggle_power() -> zbus::Result<bool> {
    let current = is_powered().await?;
    let requested = !current;

    set_powered(requested).await?;

    // Read the actual state after requesting the change.
    is_powered().await
}
