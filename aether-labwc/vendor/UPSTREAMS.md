# Vendored compositor sources

These source trees are included in full so `aether-labwc` can build without
fetching a compositor or renderer from another repository.

| Directory | Upstream | Revision | License |
| --- | --- | --- | --- |
| `labwc/` | https://github.com/grigio/labwc | `8a9166968563693fd990e684243abe4ebc4cb0b5` (`labwc-blur-0.20.2`) | GPL-2.0-only |
| `scenefx/` | https://github.com/wlrfx/scenefx | `37ccd723bef49e6891156ffafce8f549f01446cc` (`0.5`) | MIT |

The upstream license and copyright notices are retained in each source
directory. `labwc/` contains the compositor implementation and
`scenefx/` supplies the scene graph renderer with blur. The labwc Meson build
links SceneFX before wlroots because both export `wlr_scene_*` symbols.
