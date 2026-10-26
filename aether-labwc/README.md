# Aether labwc

A standalone labwc compositor build with SceneFX blur support. The complete
labwc and SceneFX sources are included in `vendor/`; Aether Shell integration
is intentionally out of scope for now.

This build implements the `ext-background-effect-v1` Wayland protocol and
installs separately from the system compositor. Applications must request a
blur region to receive blur; transparency alone is not enough.

## Window switcher

Alt+Tab and Alt+Shift+Tab use Aether's thumbnail-card OSD, styled with the
dark Aether surface and blue selection accent. It shows window previews, app
icons, and titles; labwc keeps cycling on the current workspace and focuses
the selected window when Alt is released. These are compositor defaults, so
an existing `rc.xml` or `themerc-override` can still customize them.

Super+1 through Super+0 switch directly to workspaces 1 through 10, and
Super+Shift+number sends the focused window to the matching workspace. Aether
starts with ten workspaces. The workspace OSD stays visible for 1.2 seconds
and uses a rounded Aether surface, 24x18 markers, a blue active workspace, and
a short slide-in animation. The current workspace name is shown beneath the
markers. These are built-in defaults and may be overridden by `rc.xml` or
`themerc-override`.

## Sources

- `vendor/labwc/`: labwc blur fork, commit `8a9166968563693fd990e684243abe4ebc4cb0b5`
- `vendor/scenefx/`: SceneFX 0.5, commit `37ccd723bef49e6891156ffafce8f549f01446cc`

Upstream URLs and license details are in [`vendor/UPSTREAMS.md`](vendor/UPSTREAMS.md).

## Build on Void Linux

Install the build tools and development libraries:

```sh
sudo xbps-install -S base-devel git meson ninja pkg-config \
  wayland-devel wayland-protocols wlroots0.20-devel \
  libinput-devel libdrm-devel libseat-devel libdisplay-info-devel \
  libliftoff-devel libxkbcommon-devel libxcb-devel \
  xcb-util-wm-devel xcb-util-errors-devel libxml2-devel glib-devel \
  cairo-devel pango-devel pixman-devel libpng-devel
```

Build and install to `~/.local/opt/aether-labwc`:

```sh
./scripts/build.sh
```

The build uses the vendored sources and stores Meson output in
`~/.cache/aether-labwc`. To choose a different install prefix:

```sh
AETHER_PREFIX="$HOME/.local/opt/aether-labwc" ./scripts/build.sh
```

## Add the Aether login session

Register the session with SDDM:

```sh
./scripts/install-session.sh
```

This installs the launcher at `~/.local/bin/aether-labwc` and
`/usr/share/wayland-sessions/aether.desktop`. The latter requires administrator
authorization. At the next login, choose **Aether** from SDDM's Session menu.
Installing the entry does not log out or restart the current desktop.

The launcher sets `LD_LIBRARY_PATH` and `XDG_DATA_DIRS` for the private install,
then starts its labwc binary. It does not replace `/usr/bin/labwc` or modify
system wlroots.
