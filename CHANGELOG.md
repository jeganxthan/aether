# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project aims
to follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- MIT license, contributor guide, code of conduct, security policy, issue/PR
  templates, and CI.
- Notification toast in the island with a Mochi state per severity
  (`normal` / `warning` / `critical`), smooth fade in/out.
- Volume / brightness OSD: a compact island bar with Mochi and a progress
  readout, driven by the media keys. labwc keybinds send `SIGRTMIN+0..4`
  (volume up/down/mute, brightness up/down), which the app turns into a
  passive overlay that never steals focus.

### Fixed

- The reserved top strip is now pinned to the output the island window is on,
  instead of being placed on the output under the pointer (which made it
  nondeterministic on a multi-monitor setup).
- The volume OSD no longer shows the wrong icon: the low / high speaker
  assets were swapped.

## [0.1.0]

### Added

- Island shell that expands from a transparent 28px top strip reserved via a
  `wlr-layer-shell` exclusive zone.
- App launcher with fuzzy search over `.desktop` entries.
- Clipboard history backed by SQLite.
- Wi-Fi switcher (scan / connect / disconnect).
- MPRIS media controls with album art.
- Notification capture and history.
- Battery status and clock.
- Mochi animated companion (ported from Coucou).
- labwc integration via signals (`SIGUSR2` → launcher, `SIGWINCH` → clipboard).

[Unreleased]: https://github.com/jeganxthan/aether/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jeganxthan/aether/releases/tag/v0.1.0
