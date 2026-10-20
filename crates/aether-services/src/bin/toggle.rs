use aether_services::WifiToggle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wifi = WifiToggle::new().await?;

    println!("Current Wi-Fi state: {}", wifi.is_enabled().await?);

    let new_state = wifi.toggle().await?;

    println!(
        "Wi-Fi is now {}",
        if new_state { "ON" } else { "OFF" }
    );

    Ok(())
}