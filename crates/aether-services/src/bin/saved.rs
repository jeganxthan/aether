
use nmrs::NetworkManager;
use aether_services::wifi::saved_networks::list_saved_wifi_networks;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = NetworkManager::new().await?;
    let networks = list_saved_wifi_networks(&manager).await?;

    if networks.is_empty() {
        println!("No saved Wi-Fi networks found.");
        return Ok(());
    }

    println!("Saved Wi-Fi networks: {}", networks.len());

    for (index, network) in networks.iter().enumerate() {
        println!("{}. {}", index + 1, network.id);
        println!("   UUID: {}", network.uuid);
        println!(
            "   Interface: {}",
            network.interface_name.as_deref().unwrap_or("Not specified")
        );
        println!("   Auto-connect: {}", network.autoconnect);
    }

    Ok(())
}
