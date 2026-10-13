# labwc integration

Aether targets the [labwc](https://labwc.github.io/) compositor. This directory
holds the compositor-side configuration the app expects.

| File | Purpose | Required |
| --- | --- | --- |
| [`rc.xml`](rc.xml) | Keybinds + `windowRule` fragment to merge into `~/.config/labwc/rc.xml`. | **Yes** |
| [`autostart`](autostart) | Example labwc autostart entry to launch Aether at login. | No |
| [`themerc`](themerc) | Aether's dark labwc theme (titlebars, menus, OSD). | No |

## 1. Merge the keybinds and window rule (required)

Aether is toggled with Unix signals because global shortcuts are unreliable
under Wayland. Open `~/.config/labwc/rc.xml`, back it up, and copy the
`<keyboard>` and `<windowRules>` blocks from [`rc.xml`](rc.xml) into the matching
sections of your config:

```bash
cp ~/.config/labwc/rc.xml{,.bak}
$EDITOR ~/.config/labwc/rc.xml
labwc -r        # reload
```

- `Super` + `Space` — toggle the launcher (`SIGUSR2`)
- `Super` + `C` — toggle the clipboard (`SIGRTMIN+5`)
- Volume keys — show the volume OSD (`SIGRTMIN+0` / `+1` / `+2`)
- Brightness keys — show the brightness OSD (`SIGRTMIN+3` / `+4`)

> `SIGUSR1` is reserved by WebKitGTK — never send it to the process.
> `SIGRTMIN+N` is the realtime signal `34+N`; the media-key OSD is passive and
> never takes keyboard focus.

The `<windowRules>` entry drops server-side decorations, keeps the widget out of
the taskbar / window switcher, pins it on top and moves it to the top centre.
The `MoveTo x` value centres the 560 px-wide window: `x = (output_width - 560) / 2`
(`680` for a 1920 px output).

## 2. Launch at login (optional)

Copy [`autostart`](autostart) to `~/.config/labwc/autostart` (create the file if
you do not have one) and adjust the command if the binary is not on `PATH`.

## 3. Theme (optional)

The theme is purely cosmetic — Aether works without it.

```bash
cp themerc ~/.config/labwc/themerc   # back up any existing theme first
labwc -r
```
