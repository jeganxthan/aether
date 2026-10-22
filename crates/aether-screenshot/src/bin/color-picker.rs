fn main() {
    match aether_services::pick_color() {
        Ok(color) => println!("Copied {color} to clipboard"),
        Err(e) => {
            eprintln!("Failed to pick color: {e}");
            std::process::exit(1);
        }
    }
}
