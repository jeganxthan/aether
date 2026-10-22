
use aether_services::WifiScanner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scanner = WifiScanner::new().await?;

    println!("Scanning for Wi-Fi networks...\n");

    let networks = scanner.scan_and_list().await?;

    if networks.is_empty() {
        println!("No networks found.");
        return Ok(());
    }

    for network in networks {
        let signal = network.strength.unwrap_or(0);

        let band = match network.frequency {
            Some(freq) if freq >= 5925 => "6 GHz",
            Some(freq) if freq >= 4900 => "5 GHz",
            Some(_) => "2.4 GHz",
            None => "Unknown",
        };

        println!("SSID:       {}", network.ssid);
        println!("Signal:     {}%", signal);
        println!("Frequency:  {:?} MHz", network.frequency);
        println!("Band:       {}", band);
        println!("Security:   {}", if network.secured { "Secured" } else { "Open" });
        println!("Connected:  {}", network.is_active);
        println!("BSSID:      {:?}", network.bssid);
        println!("--------------------------------");
    }

    Ok(())
}
