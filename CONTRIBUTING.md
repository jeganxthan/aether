# Contributing to Aether

Thanks for wanting to help! Aether is a small, early project, so issues, ideas,
screenshots and pull requests are all valuable.

By participating you agree to follow our
[Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to contribute

- **Report bugs** with the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md).
- **Request features** with the [feature template](.github/ISSUE_TEMPLATE/feature_request.md).
- **Pick up an issue** — look for `good first issue` and `help wanted`.
- **Improve docs** — the README, this file, and code comments all count.
- **Share it** — screenshots and demos help more than you'd think.

## Development setup

### Prerequisites

- Linux with a **Wayland** session running **labwc** (Aether uses `wlr-layer-shell`)
- Node.js 18+ and npm
- Rust stable + Cargo
- System libraries:

```bash
sudo apt install \
  libgtk-3-dev libwebkit2gtk-4.1-dev libwayland-dev libxkbcommon-dev \
  libayatana-appindicator3-dev librsvg2-dev build-essential
```

### Run it

```bash
git clone https://github.com/jeganxthan/aether.git
cd aether
npm install
npm run tauri dev
```

The island maps a transparent `wlr-layer-shell` strip at the top of the screen.
If it doesn't appear, make sure you're on a Wayland session with labwc and that
another layer surface isn't already anchored to the top edge.

### Project layout

- `src/ui/island/` — the shell: layout, input shape, and the Mochi canvas
  (`src/ui/island/mochi/`).
- `src/component/{launcher,clipboard,wifi,music,notifications}/` — panel UIs.
- `src-tauri/src/commands/` — the Tauri command surface the frontend calls.
- `src-tauri/src/services/` — system integrations (D-Bus, MPRIS, wifi, wire).
- `src-tauri/src/space.rs` — the top-strip layer surface.
- `scripts/` — standalone test scripts.

## Before you open a pull request

Run the checks locally — CI runs the same ones:

```bash
npx tsc --noEmit                          # type-check the frontend
npm test                                  # tsx regression tests
npx tsx scripts/mochiEngine.smoke.ts      # headless Mochi engine smoke test
cd src-tauri && cargo fmt && cargo clippy && cargo build
```

Then:

1. Keep changes focused; one topic per PR.
2. Describe **what** changed and **why**, and add before/after screenshots for UI
   changes.
3. Reference the issue it closes (`Closes #123`).
4. Don't commit generated output (`dist/`, `target/`) or new assets without a
   clear license.

## Style

- **TypeScript/React** — match the surrounding files; the project uses Tailwind
  utility classes and small function components.
- **Rust** — `cargo fmt` and a clean `cargo clippy`.
- No new third-party dependencies without a quick note on why in the PR.

## Licensing of contributions

Unless you state otherwise, any contribution you submit is licensed under the
project's [MIT License](LICENSE). Note that third-party assets (see
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)) are excluded — do not add
assets you don't have the rights to.
