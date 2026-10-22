# aether-shell

The Wayland shell for Void Linux: a labwc panel plus the services it drives.

## Screenshot tools

`crates/aether-screenshot` installs four helpers built on the services in
`crates/aether-services/src/screen_utils`:

| Binary | Does |
| --- | --- |
| `aether-full-screenshot` | Grabs the whole screen, saves it to `~/Pictures/Screenshots` and copies it to the clipboard |
| `aether-selective-screenshot` | Same, for a region dragged on screen |
| `aether-ocr` | Reads the text in a dragged region and copies it to the clipboard |
| `aether-color-picker` | Picks the color under the pointer and copies its hex value |

Install them with:

```bash
./crates/aether-screenshot/install.sh
```

The script builds them in release mode and copies them into `~/.local/bin`,
then prints the keybinds to paste into `~/.config/labwc/rc.xml`:

```
Super + S           screenshot
Super + Shift + S   selective screenshot
Super + Shift + O   select text (OCR)
Super + Shift + C   color picker
```

Run `labwc -r` afterwards to reload the compositor config.

The helpers need `grim`, `slurp`, `tesseract` and `wl-copy` on the system:

```bash
sudo xbps-install grim slurp tesseract-ocr
```

Calling `cargo run` from the keybinds is avoided on purpose: it rebuilds on
every press, prints build noise over the desktop and blocks on the cargo lock.
