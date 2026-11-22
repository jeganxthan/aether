use nmrs::NetworkManager;

async fn  main(){
    let nm = NetworkManager::new().await?;
    let devices = nm.list_devices().await?;
    
    for device in devices {
        println!("Interface: {}", device.interface);
        println!("  Type: {}", device.device_type);
        println!("  State: {}", device.state);
        println!("  MAC: {}", device.identity.current_mac);
        
        if device.is_wireless() {
            println!("  This is a WiFi device");
        } else if device.is_wired() {
            println!("  This is an Ethernet device");
        } else if device.is_bluetooth() {
            println!("  This is a Bluetooth device");
        } else if device.is_loopback() {
            println!("  This is a loopback device");
        }
        
        if let Some(driver) = &device.driver {
            println!("  Driver: {}", driver);
        }
    }
}