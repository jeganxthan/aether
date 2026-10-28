# Aether

**A Rust-powered Wayland desktop, built around labwc.**

Aether brings together a lightweight labwc-based Wayland session and a native
Rust desktop shell. The shell adds a panel, settings, notifications, quick
controls, and on-screen feedback while labwc handles window management and
compositing.

> **Project status:** Aether is under active development. Features and setup
> steps may change as the project evolves.

**[Explore the landing page](https://jeganxthan.github.io/aether/) ·
[Browse the source](https://github.com/jeganxthan/aether)**

## What is Aether?

Aether is a Linux desktop project made of cooperating parts, rather than a
replacement window manager written from scratch:

- **Aether Shell** is the Rust/Wayland desktop shell: a panel, popups,
  notifications, media controls, and animated on-screen displays.
- **Aether UI** is the Rust widget and rendering layer used by the shell.
- **Aether Services** provides system integrations for things such as audio,
  brightness, Wi-Fi, Bluetooth, battery status, and screenshots.
- **Aether Settings** is a Tauri application for desktop settings.
- **Aether labwc** is a separately built labwc-based compositor/session, with
  SceneFX support. labwc itself is not written in Rust.

The repository includes the compositor and SceneFX sources under
[`aether-labwc/vendor`](./aether-labwc/vendor). Their upstream projects,
revisions, and license details are documented in
[`aether-labwc/vendor/UPSTREAMS.md`](./aether-labwc/vendor/UPSTREAMS.md).

## Highlights

- Wayland desktop shell designed to run with labwc.
- Panel and quick-access popups for Wi-Fi, Bluetooth, battery, notifications,
  media, and settings.
- Volume, screen-brightness, microphone-mute, and keyboard-backlight OSDs.
- Light and dark shell themes.
- Rust services for desktop integrations and utilities.
- An optional, separately installed labwc + SceneFX session.

## Project layout

| Path | Purpose |
| --- | --- |
| `crates/aether-shell` | Rust desktop shell and its panel |
| `crates/aether-ui` | Wayland UI toolkit and rendering |
| `crates/aether-services` | System and desktop services |
| `crates/aether-settings` | Tauri settings application |
| `aether-labwc` | Vendored labwc/SceneFX sources and session scripts |
| `assets` | Icons and other shell assets |
| `docs` | Project landing page published with GitHub Pages |

## Getting started

Aether targets Linux and Wayland. The compositor build instructions are
currently written for Void Linux; the exact system packages may differ on
other distributions.

### Build the Rust shell

Install a recent Rust toolchain and the system development libraries required
by the Wayland, graphics, and audio dependencies, then build the shell:

```sh
cargo build --release -p aether-shell
```

To run it from an existing Wayland session:

```sh
cargo run --release -p aether-shell
```

The shell expects a working Wayland compositor and desktop services on the
system. It is not a standalone compositor.

### Build the Aether labwc session

The compositor build has its own distribution-specific dependencies and setup
steps. Follow the [Aether labwc guide](./aether-labwc/README.md) to install
build tools, compile the vendored labwc/SceneFX sources, and register the
session with SDDM.

## Contributing

Aether is evolving. Issues and focused pull requests are welcome. Before
proposing a change, check the relevant crate or the
[labwc setup guide](./aether-labwc/README.md) to understand which part of the
desktop it affects.

## License

The repository contains multiple components with their own licensing terms.
Check the relevant component and the upstream notices in
[`aether-labwc/vendor/UPSTREAMS.md`](./aether-labwc/vendor/UPSTREAMS.md)
before redistributing or reusing code.
