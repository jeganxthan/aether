fn main() {
    match aether_services::selective_screenshot() {
        Ok(path) => println!("Saved screenshot to {}", path.display()),
        Err(e) => {
            eprintln!("Failed to take selective screenshot: {e}");
            std::process::exit(1);
        }
    }
}
