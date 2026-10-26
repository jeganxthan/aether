use std::time::Duration;

use aether_services::bluetooth::{devices, power};

#[tokio::main]
async fn main() {
    println!("=== Aether Bluetooth Test ===\n");

    match power::is_powered().await {
        Ok(true) => println!("Bluetooth powered: true"),
        Ok(false) => {
            eprintln!("Bluetooth is powered off.");
            return;
        }
        Err(error) => {
            eprintln!("Power check failed: {error}");
            return;
        }
    }

    let session = match devices::DiscoverySession::start().await {
        Ok(session) => {
            println!("Discovery started: {}", session.adapter_path);
            session
        }
        Err(error) => {
            eprintln!("Could not start discovery: {error}");
            return;
        }
    };

    println!("Scanning for 10 seconds...");
    tokio::time::sleep(Duration::from_secs(10)).await;

    println!("\n--- Discovered Devices ---");

    match session.list_devices().await {
        Ok(found) if found.is_empty() => {
            println!("No devices returned by BlueZ.");
        }
        Ok(found) => {
            println!("Found {} device(s):\n", found.len());

            for (i, device) in found.iter().enumerate() {
                println!("{}. {}", i + 1, device.name);
                println!("   Address:   {}", device.address);
                println!("   Path:      {}", device.path);
                println!("   Paired:    {}", device.paired);
                println!("   Connected: {}", device.connected);
                println!("   Trusted:   {}", device.trusted);
                println!(
                    "   RSSI:      {}",
                    device
                        .rssi
                        .map(|rssi| format!("{rssi} dBm"))
                        .unwrap_or_else(|| "unavailable".into())
                );
                println!();
            }
        }
        Err(error) => eprintln!("Listing failed: {error}"),
    }

    match session.stop().await {
        Ok(()) => println!("Discovery stopped."),
        Err(error) => eprintln!("Stop discovery failed: {error}"),
    }

    println!("=== Test Complete ===");
}
