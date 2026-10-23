#!/usr/bin/env bash
#
# Builds the screenshot helpers and installs them into ~/.local/bin so the
# labwc keybinds can call them directly.
#
# Calling `cargo run` from the keybinds is a bad idea: it rebuilds on every
# press, prints build noise over the desktop and blocks on the cargo lock.
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local/bin}"
DATA_DIR="${DATA_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}}"
ICON_DIR="$DATA_DIR/aether/icons"

BINARIES=(
    "full-screenshot:aether-full-screenshot"
    "selective-screenshot:aether-selective-screenshot"
    "ocr-selection:aether-ocr"
    "color-picker:aether-color-picker"
)

# the icons the notifications point at, by the name the tools ask for
ICONS=(
    "assets/icons/common/video-camera-svgrepo-com.svg:screenshot.svg"
    "assets/icons/common/color-picker.svg:color-picker.svg"
    "assets/icons/launcher/clipboard.svg:clipboard.svg"
)

echo "==> Building aether-screenshot (release)"
cargo build --release --manifest-path "$ROOT/Cargo.toml" -p aether-screenshot

mkdir -p "$PREFIX"
for entry in "${BINARIES[@]}"; do
    src="${entry%%:*}"
    dst="${entry##*:}"
    install -m 0755 "$ROOT/target/release/$src" "$PREFIX/$dst"
    echo "    installed $PREFIX/$dst"
done

mkdir -p "$ICON_DIR"
for entry in "${ICONS[@]}"; do
    src="${entry%%:*}"
    dst="${entry##*:}"
    install -m 0644 "$ROOT/$src" "$ICON_DIR/$dst"
    echo "    installed $ICON_DIR/$dst"
done

missing=()
for tool in grim slurp wl-copy notify-send; do
    command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done

# Void installs the engine as tesseract-ocr, while other distros use tesseract
ocr=""
for tool in tesseract tesseract-ocr; do
    if command -v "$tool" >/dev/null 2>&1; then
        ocr="$tool"
        break
    fi
done

if [[ -z "$ocr" ]]; then
    missing+=("tesseract-ocr")
elif ! "$ocr" --list-langs 2>/dev/null | grep -qx "eng"; then
    # the engine alone ships no English data, so OCR would fail without it
    missing+=("tesseract-ocr-eng")
fi

if ((${#missing[@]})); then
    echo
    echo "==> Missing runtime tools: ${missing[*]}"
    echo "    sudo xbps-install grim slurp wl-copy"
    echo "    sudo xbps-install tesseract-ocr tesseract-ocr-eng"
fi

cat <<EOF

==> labwc keybinds (add these to ~/.config/labwc/rc.xml)

    <keybind key="W-s"><action name="Execute" command="$PREFIX/aether-full-screenshot" /></keybind>
    <keybind key="W-S-s"><action name="Execute" command="$PREFIX/aether-selective-screenshot" /></keybind>
    <keybind key="W-S-o"><action name="Execute" command="$PREFIX/aether-ocr" /></keybind>
    <keybind key="W-S-c"><action name="Execute" command="$PREFIX/aether-color-picker" /></keybind>

    then reload with: labwc -r
EOF
