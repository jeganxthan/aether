use aether_services::wifi::WifiManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wifi = WifiManager::new().await?;
    wifi.force_enable().await?;
    println!("Wi-Fi enabled ");
    Ok(())
}
