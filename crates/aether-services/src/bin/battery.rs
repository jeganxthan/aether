use aether_services::battery::percentage::BatteryService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let battery = BatteryService::new()?;
    let mut receiver = battery.subscribe();

    println!("Watching battery changes...");

    loop {
        receiver.changed().await?;

        match receiver.borrow_and_update().clone() {
            Some(info) => println!("Battery: {}%", info.percentage),
            None => println!("Battery reading unavailable"),
        }
    }
}