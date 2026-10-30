#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PREFIX=${AETHER_PREFIX:-"$HOME/.local/opt/aether-labwc"}
LAUNCHER="$HOME/.local/bin/aether-labwc"
SESSION_DIR=${AETHER_SESSION_DIR:-/usr/share/wayland-sessions}

if [ ! -x "$PREFIX/bin/labwc" ]; then
	printf 'Aether labwc binary not found at %s/bin/labwc; run scripts/build.sh first.\n' "$PREFIX" >&2
	exit 1
fi

mkdir -p "$HOME/.local/bin"
sed "s|@AETHER_PREFIX@|$PREFIX|g" "$SCRIPT_DIR/bin/aether-labwc.in" > "$LAUNCHER"
chmod 755 "$LAUNCHER"

desktop_tmp=$(mktemp)
trap 'rm -f "$desktop_tmp"' EXIT HUP INT TERM
sed "s|@AETHER_LAUNCHER@|$LAUNCHER|g" \
	"$SCRIPT_DIR/share/wayland-sessions/aether.desktop.in" > "$desktop_tmp"

desktop_target="$SESSION_DIR/aether.desktop"
if [ -w "$SESSION_DIR" ]; then
	install -D -m 0644 "$desktop_tmp" "$desktop_target"
else
	if ! command -v sudo >/dev/null 2>&1; then
		printf 'Cannot install %s: run this script with sudo available or set AETHER_SESSION_DIR.\n' "$desktop_target" >&2
		exit 1
	fi
	sudo install -D -m 0644 "$desktop_tmp" "$desktop_target"
fi

if [ "$SESSION_DIR" = /usr/share/wayland-sessions ] &&
	[ -f /usr/share/wayland-sessions/labwc-blur.desktop ]; then
	if [ -w /usr/share/wayland-sessions ]; then
		rm -f /usr/share/wayland-sessions/labwc-blur.desktop
	else
		sudo rm -f /usr/share/wayland-sessions/labwc-blur.desktop
	fi
fi

printf 'Installed the Aether session: %s\n' "$desktop_target"
labwc_config_dir="$HOME/.config/labwc"
labwc_config="$labwc_config_dir/rc.xml"
if [ ! -e "$labwc_config" ]; then
	mkdir -p "$labwc_config_dir"
	install -m 0644 "$SCRIPT_DIR/config/rc.xml" "$labwc_config"
	printf 'Installed default Aether clipboard shortcuts in %s\n' "$labwc_config"
else
	printf 'Preserved existing labwc config: %s\n' "$labwc_config"
fi
printf 'Select Aether from the SDDM Session menu at your next login.\n'
