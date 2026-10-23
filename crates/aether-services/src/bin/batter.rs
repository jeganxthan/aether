
#[path = "../battery/info.rs"]
mod info;

use info::{battery, format_duration};

fn main() {
    println!("=== Aether Battery Service Test ===\n");

    match battery() {
        Some(battery) => {
            println!("Battery detected successfully!");
            println!("---------------------------------");
            println!("Percentage : {}%", battery.percentage);
            println!("Status     : {:?}", battery.status);
            println!("Charging   : {}", battery.is_charging());
            println!("Discharging: {}", battery.is_discharging());

            println!("\nTime estimates");
            println!("---------------------------------");

            match battery.remaining_time {
                Some(duration) => {
                    println!("Remaining time : {}", format_duration(duration));
                    println!(
                        "Remaining secs : {} seconds",
                        duration.as_secs()
                    );
                }
                None => println!("Remaining time : Unavailable"),
            }

            match battery.time_to_full {
                Some(duration) => {
                    println!("Time to full   : {}", format_duration(duration));
                    println!(
                        "Charging secs  : {} seconds",
                        duration.as_secs()
                    );
                }
                None => println!("Time to full   : Unavailable"),
            }
        }
        None => {
            eprintln!("No battery detected or battery data could not be read.");
            std::process::exit(1);
        }
    }
}
