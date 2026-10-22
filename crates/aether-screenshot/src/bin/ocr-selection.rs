fn main() {
    match aether_services::ocr_selection() {
        Ok(text) => println!("Copied text to clipboard:\n{text}"),
        Err(e) => {
            eprintln!("Failed to OCR selection: {e}");
            std::process::exit(1);
        }
    }
}
