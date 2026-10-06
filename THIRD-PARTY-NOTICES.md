# Third-party notices

## Mochi character and animation — NotchBuddy / Coucou

The Mochi character rendering, animation timing and greeting sequence in
`src/ui/island/mochi/` are ported from
[NotchBuddy / Coucou](https://github.com/Louis-CFM/coucou) by Louis Raillé
(`NotchBuddy/Sources/App/BotEngine.swift`, `IslandRootView.swift`,
`IslandTypes.swift`).

Coucou's source code is MIT licensed:

```
MIT License

Copyright (c) 2026 Louis Raillé
```

The Mochi character artwork itself is **not** covered by that MIT licence — see
Coucou's `LICENSE-ASSETS.md`. It is used here under separate permission granted
by the copyright holder. The names "Coucou" and "Mochi", the app and menu bar
icons, and the sound effects remain the property of Louis Raillé and are not
included in this repository.

If this permission is withdrawn, `src/ui/island/mochi/` must be replaced with a
character of your own; nothing else in the app depends on its internals beyond
the `MochiCanvas` component's `expanded` prop.

### What was changed in the port

- SwiftUI `GraphicsContext` drawing → Canvas 2D.
- `DispatchQueue.main.asyncAfter` scheduling → a pending-callback queue drained
  in `MochiEngine.update`, so timing is driven by the render loop.
- `CACurrentMediaTime()` → `performance.now()`.
- SwiftUI `TimelineView` → `requestAnimationFrame`.
- The upload/mailbox morph (`morph`, mouth slot spring, `cup` eyes, badge while
  morphing) and the mini-bot variant were dropped — Aether has no file-drop or
  agent-pill surface for them. The mailbox morph maths is untouched and can be
  reintroduced if that surface appears.
- The greeting timeline, hands, waves, emotes, particles, blinking, look-at and
  every eye shape are ported unchanged.
