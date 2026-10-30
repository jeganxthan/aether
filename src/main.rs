fn main() {
    println!("Hello, world!");
}
// wlr-randr --output HDMI-A-1 --pos 0,0 --output eDP-1 --pos 1920,0

// cd ~/Desktop/config/aether-shell
// cargo build --release -p aether-panel

// install -Dm755 \
//   target/release/aether-panel \
//   ~/.local/bin/aether-panel


// # 1. Build the latest release
// cargo build --release -p aether-shell

// # 2. Verify the new executable
// ls -lh target/release/aether-shell

// # 3. Stop the currently running shell
// pkill -x aether-shell 2>/dev/null || true

// # 4. Install the updated build
// install -Dm755 target/release/aether-shell ~/.local/bin/aether-shell

// # 5. Launch the updated shell
// ~/.local/bin/aether-shell