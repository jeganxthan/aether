fn main() {
    match aether_services::full_screenshot() {
        Ok(path) => println!("Saved screenshot to {}", path.display()),
        Err(e) => {
            eprintln!("Failed to take screenshot: {e}");
            std::process::exit(1);
        }
    }
}
