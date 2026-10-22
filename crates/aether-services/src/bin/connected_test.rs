
use aether_services::ConnectedWifiService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = ConnectedWifiService::new().await?;

    match service.get().await? {
        None => println!("Not connected to Wi-Fi."),
        Some(info) => {
            let n = &info.network;

            println!("--- Connected Wi-Fi ---");
            println!("SSID:       {}", n.ssid);
            println!("BSSID:      {}", n.bssid);
            println!("Signal:     {}%", n.strength);
            println!("Signal bars: {}", n.bars);
            println!("Frequency:  {:?} MHz", n.freq);
            println!("Channel:    {:?}", n.channel);
            println!("Mode:       {}", n.mode);
            println!("Security:   {}", n.security);
            println!("Status:     {}", n.status);
            println!("Link rate:  {:?} Mbps", n.rate_mbps);
            println!("IPv4:       {:?}", n.ip4_address);
            println!("IPv6:       {:?}", n.ip6_address);

            if let Some(device) = &info.device {
                println!("\n--- Wi-Fi Adapter ---");
                println!("Interface:  {}", device.interface);
                println!("MAC:        {}", device.hw_address);
                println!("Permanent MAC: {:?}", device.permanent_hw_address);
                println!("Driver:     {:?}", device.driver);
                println!("State:      {:?}", device.state);
                println!("Managed:    {}", device.managed);
                println!("Autoconnect: {}", device.autoconnect);
                println!("Frequency:  {:?} MHz", device.active_frequency_mhz);
            }
        }
    }

    Ok(())
}
