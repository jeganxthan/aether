#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

STATE_DIR=${AETHER_STATE_DIR:-"${XDG_CACHE_HOME:-$HOME/.cache}/aether-labwc"}
BUILD_DIR="$STATE_DIR/build"
PREFIX=${AETHER_PREFIX:-"$HOME/.local/opt/aether-labwc"}
SCENEFX_SOURCE=${AETHER_SCENEFX_SOURCE:-"$SCRIPT_DIR/vendor/scenefx"}
LABWC_SOURCE=${AETHER_LABWC_SOURCE:-"$SCRIPT_DIR/vendor/labwc"}

for command in meson ninja pkg-config; do
	if ! command -v "$command" >/dev/null 2>&1; then
		printf 'Required build tool not found: %s\n' "$command" >&2
		exit 1
	fi
done

for source_dir in "$SCENEFX_SOURCE" "$LABWC_SOURCE"; do
	if [ ! -f "$source_dir/meson.build" ]; then
		printf 'Vendored Meson source not found: %s\n' "$source_dir" >&2
		exit 1
	fi
done

configure_build() {
	build_dir=$1
	source_dir=$2
	shift 2

	if [ -f "$build_dir/meson-private/coredata.dat" ]; then
		meson setup --reconfigure "$build_dir" "$source_dir" "$@"
	else
		mkdir -p "$(dirname "$build_dir")"
		meson setup "$build_dir" "$source_dir" "$@"
	fi
}

mkdir -p "$PREFIX"

scenefx_build="$BUILD_DIR/scenefx"
labwc_build="$BUILD_DIR/labwc"

configure_build "$scenefx_build" "$SCENEFX_SOURCE" \
	--prefix="$PREFIX" \
	--libdir=lib \
	-Dexamples=false \
	-Dtracy_enable=false \
	-Dcolor-management=disabled
meson compile -C "$scenefx_build"
meson install -C "$scenefx_build"

export PKG_CONFIG_PATH="$PREFIX/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
configure_build "$labwc_build" "$LABWC_SOURCE" \
	--prefix="$PREFIX" \
	--libdir=lib \
	-Dman-pages=disabled \
	-Dsvg=disabled \
	-Dicon=disabled \
	-Dsystemd-session=disabled \
	-Dxwayland=auto \
	-Dnls=auto \
	-Dlabnag=auto
meson compile -C "$labwc_build"
meson install -C "$labwc_build"

printf 'Aether labwc and SceneFX installed under %s\n' "$PREFIX"
printf 'Next: run scripts/install-session.sh to register the Aether SDDM session.\n'
