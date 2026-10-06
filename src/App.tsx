import Island from "./ui/island/Island";

export default function App() {
  return <Island />;
}

// ochi has a layered expression system, not a single "expression" field. Three independent channels combine:
// Channel	Field	Role
// Eyes	EyeShape (12 shapes)	The main expression carrier
// Body	pose tweens	squash, stretch, tilt, bounce
// Accents	particles, blush, badge	decorative reinforcement
// Eye shapes (mochiEngine.ts:10): pill (neutral default), wide, dot, line, flat, happy, closed, spiral, heart, star, tired, wink. All 12 have a real rendering case in drawEyeShape — none are stubs.
// Emotes (setEmote, mochiEngine.ts:272) — 7, each with full choreography:
// - love → heart eyes, blush to full, 4 heart particles, small upward hop
// - surprised → dot eyes, body lifts (oy -0.3) with a back overshoot, es eye-stretch 1.25×
// - proud → star eyes, 5 star particles, holds a −0.14 tilt, blush 0.7
// - wink → wink eye, holds a +0.12 tilt
// - yawn → stretches tall/narrow over 500ms, then eyes go closed + 2 z particles at t=0.7
// - happy → happy eyes + blush 0.6
// - annoyed → flat line eyes for a short 0.8s
// Badge (4): dots, bang, question, dot — each with a color, tweened via badgeS.
// Particles (5): heart, star, spark, sweat, z (drawn as literal text).
// Idle behavior on top: random blink() scheduling, and squash() as a reusable impact reaction.
// The greeting
// greet() is a timed multi-beat routine rather than a single emote: happy eyes for 2s → hands pop out at 0.25s with a squash → right hand waves 0.55–1.55s → blinks at 0.55s and 1.5s → hands retract at 1.55s → a final 0.3s happy-eye flash at 1.75s.