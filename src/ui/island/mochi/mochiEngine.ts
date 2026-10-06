// Mochi engine — ported from NotchBuddy/coucou `BotEngine.swift`.
//
// MIT License, Copyright (c) 2026 Louis Raillé. Portions of the drawing maths,
// easing curves and tween system below are derived from that project; the original
// notice is retained in the repository's THIRD-PARTY-NOTICES.md.
//
// The character art itself ("Mochi") remains the property of its author and is
// used here under separate permission — see THIRD-PARTY-NOTICES.md.

export type EyeShape =
    | "pill"
    | "wide"
    | "dot"
    | "line"
    | "flat"
    | "happy"
    | "closed"
    | "spiral"
    | "heart"
    | "star"
    | "tired"
    | "wink";

export type Emote =
    | "love"
    | "surprised"
    | "proud"
    | "wink"
    | "yawn"
    | "happy"
    | "annoyed";

export type Badge =
    | { kind: "dots"; color: RGB }
    | { kind: "bang"; color: RGB }
    | { kind: "question"; color: RGB }
    | { kind: "dot"; color: RGB }
    | { kind: "note"; color: RGB };

export type RGB = readonly [number, number, number];

type ParticleType = "heart" | "star" | "spark" | "sweat" | "z" | "note";

interface Particle {
    type: ParticleType;
    x: number;
    y: number;
    vx: number;
    vy: number;
    age: number;
    life: number;
    rot: number;
    size: number;
}

// MARK: - Easing (mirrors the prototype's E.out / E.inOut / E.back / E.lin)

const out = (t: number) => 1 - Math.pow(1 - t, 3);
const inOut = (t: number) =>
    t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
const back = (t: number) => {
    const c1 = 1.7;
    const c3 = c1 + 1;
    return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
};
const lin = (t: number) => t;

type Ease = (t: number) => number;

interface TweenKey {
    target: number;
    /** milliseconds */
    duration: number;
    ease: Ease;
}

interface Tween {
    property: string;
    keys: TweenKey[];
    keyIndex: number;
    from: number;
    /** performance.now() */
    startTime: number;
    onComplete?: () => void;
}

// MARK: - Colour helpers

const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

const mix3 = (a: RGB, b: RGB, t: number): RGB => [
    lerp(a[0], b[0], t),
    lerp(a[1], b[1], t),
    lerp(a[2], b[2], t),
];

const css = (c: RGB, alpha = 1) =>
    `rgba(${Math.round(c[0] * 255)}, ${Math.round(c[1] * 255)}, ${Math.round(
        c[2] * 255
    )}, ${alpha})`;

// MARK: - Mochi track constants (from PISTES.mochi)

const MOCHI = {
    eyeW: 0.25,
    eyeH: 0.27,
    eyeSp: 0.37,
    eyeP: -0.12,
    baseTop: [0.929, 0.929, 0.937] as RGB, // #EDEDEF
    baseBottom: [0.769, 0.773, 0.792] as RGB, // #C4C5CA
    ink: [0.102, 0.082, 0.071] as RGB, // #1A1412
};

const PINK: RGB = [1, 0.471, 0.588];
const HEART: RGB = [1, 0.302, 0.427];
const STAR: RGB = [0.969, 0.702, 0.169];
const SWEAT: RGB = [0.486, 0.78, 1];
const Z_INK: RGB = [0.82, 0.86, 0.92];
const IDLE_COLOR: RGB = [0.902, 0.914, 0.933];
const SEARCHING_COLOR: RGB = [0.388, 0.396, 0.949];
const DIZZY_COLOR: RGB = [0.957, 0.447, 0.714];
const NOTE_COLOR: RGB = [0.639, 0.902, 0.208];

// MARK: - Engine

type PropertyKey =
    | "yaw"
    | "pitch"
    | "roll"
    | "tilt"
    | "open"
    | "sx"
    | "sy"
    | "oy"
    | "ox"
    | "tint"
    | "hands"
    | "blush"
    | "es"
    | "badgeS"
    | "glow";

/**
 * Per-frame state and drawing for the Mochi character.
 *
 * Owns no timers: everything time-based is either a tween (advanced by `update`)
 * or a deadline compared against the clock value passed into `update`. That keeps
 * the animation frame-rate independent and free of `setTimeout` bookkeeping.
 */
export class MochiEngine {
    // Animation state (mirrors the prototype's `s` object)
    yaw = 0;
    pitch = 0;
    roll = 0;
    tilt = 0;
    open = 1;
    sx = 1;
    sy = 1;
    oy = 0;
    ox = 0;
    tint = 0;
    hands = 0;
    blush = 0;
    es = 1;
    badgeS = 0;

    // Smoothed targets
    private tgYaw = 0;
    private tgPitch = 0;
    private tgTilt = 0;
    private tgSy = 1;
    private tgSx = 1;

    /** Look target in -1..1, fed in from the pointer position each frame. */
    lookX = 0;
    lookY = 0;

    // Blink / wave scheduling
    private nextBlink = 0;
    private waveStart = 0;
    private waveUntil = 0;
    

    private tweens = new Map<string, Tween>();
    private locks = new Set<string>();
    private particles: Particle[] = [];

    private badge: Badge | null = null;
    private badgeKey = "none";

    constructor() {
        const now = performance.now() / 1000;
        this.nextBlink = now + 1.5 + Math.random() * 2;
    }

    // MARK: Public API

    /**
     * The greeting: eyes go happy, hands pop out, the right hand waves, then
     * everything settles back. Timings match the original `greet()`.
     */
    greet() {
        const now = performance.now() / 1000;

        this.waveStart = now + 0.45;
        this.waveUntil = now + 1.55;

        // Happy eyes for the first 2s — no gap, no flicker.
        this.eyeOverride = "happy";
        this.eyeOverrideUntil = now + 2.0;
        this.anim("oy", [
            { target: -0.06, duration: 220, ease: out },
            { target: 0.0, duration: 220, ease: back },
        ]);

        // 0.25s: hands out, body squash.
        this.animAt(0.25, () => {
            this.anim("hands", [{ target: 1, duration: 280, ease: out }]);
            this.anim("sy", [
                { target: 0.95, duration: 100, ease: out },
                { target: 1.0, duration: 260, ease: back },
            ]);
            this.anim("sx", [
                { target: 1.04, duration: 100, ease: out },
                { target: 1.0, duration: 260, ease: back },
            ]);
        });

        this.animAt(0.55, () => this.blink());
        this.animAt(1.5, () => this.blink());

        // 1.55s: hands retract.
        this.animAt(1.55, () => {
            this.waveUntil = 0;
            this.anim("hands", [{ target: 0, duration: 200, ease: inOut }]);
        });

        // 1.75s: a last flash of happy eyes.
        this.animAt(1.75, () => {
            this.eyeOverride = "happy";
            this.eyeOverrideUntil = performance.now() / 1000 + 0.3;
        });
    }

    /** Interrupt an in-flight greeting; hands retract in 150ms. */
    interruptGreet() {
        const now = performance.now() / 1000;
        if (this.hands <= 0.01 && now >= this.waveUntil) return;
        this.pending = [];
        this.waveUntil = 0;
        this.waveStart = 0;
        this.anim("hands", [{ target: 0, duration: 150, ease: inOut }]);
    }

    blink() {
        if (this.locks.has("open")) return;
        this.anim("open", [
            { target: 0.06, duration: 70, ease: inOut },
            { target: 1, duration: 130, ease: out },
        ]);
    }

    squash() {
        this.anim("sy", [
            { target: 0.78, duration: 70, ease: out },
            { target: 1.1, duration: 130, ease: out },
            { target: 1, duration: 170, ease: inOut },
        ]);
        this.anim("sx", [
            { target: 1.16, duration: 70, ease: out },
            { target: 0.95, duration: 130, ease: out },
            { target: 1, duration: 170, ease: inOut },
        ]);
    }

    /** Swallow — the slot opens, the body stretches, then it settles. */
    gulp() {
        this.anim("oy", [
            { target: 0.34, duration: 200, ease: out },
            { target: 0, duration: 260, ease: inOut },
        ]);
        this.anim("sy", [
            { target: 0.78, duration: 80, ease: out },
            { target: 1.18, duration: 130, ease: out },
            { target: 1, duration: 220, ease: back },
        ]);
        this.anim("sx", [
            { target: 1.28, duration: 80, ease: out },
            { target: 0.92, duration: 130, ease: out },
            { target: 1, duration: 220, ease: back },
        ]);
        this.blink();
    }

    poke() {
        // Roughly one tap in three swallows instead of squashing.
        if (Math.random() < 0.3) {
            this.gulp();
            return;
        }

        this.squash();

        // Dizzy escalation disabled for now.
        // const now = performance.now() / 1000;
        // if (now < this.dizzyUntil) return;
        // this.slapTimes = this.slapTimes.filter((t) => now - t < 1.7);
        // this.slapTimes.push(now);
        // if (this.slapTimes.length >= 3) {
        //     this.slapTimes = [];
        //     this.slap();
        // }
    }

    slap() {
        this.interruptGreet();
        const now = performance.now() / 1000;
        this.eyeOverride = "line";
        this.eyeOverrideUntil = now + 0.7;
        this.animAt(0.7, () => this.dizzy());
    }

    private dizzy() {
        const now = performance.now() / 1000;
        this.eyeOverride = "spiral";
        this.eyeOverrideUntil = now + 2.4;
        this.dizzyUntil = now + 3.0;
        this.setTintColor(DIZZY_COLOR, 0.7);
        this.roll = 0;
        this.anim(
            "roll",
            [{ target: Math.PI * 2 * 2, duration: 1300, ease: inOut }],
            () => {
                this.roll = 0;
            },
        );
        this.animAt(3.0, () => this.setTintColor(IDLE_COLOR, 0));
    }

    setEmote(emote: Emote, duration = 1.8) {
        const now = performance.now() / 1000;
        this.eyeOverride = emoteEyeShape(emote);
        this.eyeOverrideUntil = now + duration;

        switch (emote) {
            case "love":
                this.anim("blush", [
                    { target: 1, duration: 300, ease: out },
                    { target: 1, duration: (duration - 0.6) * 1000, ease: lin },
                    { target: 0, duration: 300, ease: inOut },
                ]);
                this.emit("heart", 4);
                this.anim("oy", [
                    { target: -0.1, duration: 160, ease: out },
                    { target: 0, duration: 300, ease: back },
                ]);
                break;
            case "surprised":
                this.anim("oy", [
                    { target: -0.3, duration: 140, ease: out },
                    { target: 0, duration: 380, ease: back },
                ]);
                this.anim("es", [
                    { target: 1.25, duration: 120, ease: out },
                    { target: 1, duration: 500, ease: inOut },
                ]);
                break;
            case "proud":
                this.emit("star", 5);
                this.anim("tilt", [
                    { target: -0.14, duration: 220, ease: out },
                    { target: -0.14, duration: (duration - 0.5) * 1000, ease: lin },
                    { target: 0, duration: 280, ease: inOut },
                ]);
                this.anim("blush", [
                    { target: 0.7, duration: 250, ease: out },
                    { target: 0.7, duration: (duration - 0.5) * 1000, ease: lin },
                    { target: 0, duration: 300, ease: inOut },
                ]);
                break;
            case "wink":
                this.anim("tilt", [
                    { target: 0.12, duration: 160, ease: out },
                    { target: 0.12, duration: (duration - 0.4) * 1000, ease: lin },
                    { target: 0, duration: 240, ease: inOut },
                ]);
                break;
            case "yawn":
                this.anim("sy", [
                    { target: 1.12, duration: 500, ease: inOut },
                    { target: 1, duration: 500, ease: inOut },
                ]);
                this.anim("sx", [
                    { target: 0.94, duration: 500, ease: inOut },
                    { target: 1, duration: 500, ease: inOut },
                ]);
                this.animAt(0.7, () => {
                    this.eyeOverride = "closed";
                    this.emit("z", 2);
                });
                break;
            case "happy":
                this.anim("blush", [
                    { target: 0.6, duration: 200, ease: out },
                    { target: 0, duration: 600, ease: inOut },
                ]);
                break;
            case "annoyed":
                this.eyeOverride = "line";
                this.eyeOverrideUntil = now + 0.8;
                break;
        }
    }

    /** Tint the body toward a state colour, as the original does for each BotState. */
    setTintColor(color: RGB, amount: number) {
        this.tintTarget = amount;
        this.tintColor = color;
    }

    setSearching(searching: boolean) {
        if (this.searching === searching) return;
        this.searching = searching;
        if (searching) {
            this.setTintColor(SEARCHING_COLOR, 0.72);
            this.setBadge({ kind: "dots", color: SEARCHING_COLOR });
        } else {
            this.setTintColor(IDLE_COLOR, 0);
            this.setBadge(null);
        }
    }

    /**
     * Music playing elsewhere: green body, closed eyes, headphones and a
     * tempo sway. Searching outranks it, since both want the tint and badge.
     */
    setListening(listening: boolean) {
        if (this.listening === listening) return;
        if (listening && this.searching) return;

        this.listening = listening;

        if (listening) {
            this.setTintColor(NOTE_COLOR, 0.72);
            this.setBadge({ kind: "note", color: NOTE_COLOR });
            this.lastNote = performance.now() / 1000;
            this.emit("note", 2);
        } else if (this.searching) {
            this.setTintColor(SEARCHING_COLOR, 0.72);
            this.setBadge({ kind: "dots", color: SEARCHING_COLOR });
        } else {
            this.setTintColor(IDLE_COLOR, 0);
            this.setBadge(null);
        }
    }

    setBadge(badge: Badge | null) {
        const key = badgeKeyOf(badge);
        if (key === this.badgeKey) return;
        this.badgeKey = key;
        const token = ++this.badgeToken;
        this.anim("badgeS", [{ target: 0, duration: 90, ease: inOut }]);
        this.animAt(0.1, () => {
            if (token !== this.badgeToken) return;
            this.badge = badge;
            if (badge) {
                this.anim("badgeS", [
                    { target: 1, duration: 280, ease: back },
                ]);
            }
        });
    }

    emit(type: ParticleType, count: number) {
        const rand = (a: number, b: number) => a + Math.random() * (b - a);

        for (let i = 0; i < count; i++) {
            const isZ = type === "z";
            this.particles.push({
                type,
                x: rand(-0.5, 0.5) * 0.9 + (isZ ? 0.55 : 0),
                y: -0.7 - rand(0, 0.2),
                vx: rand(-0.5, 0.5) * 0.35 + (isZ ? 0.18 : 0),
                vy: -(0.45 + rand(0, 0.35)),
                age: -i * 0.14,
                life: rand(1.3, 1.8),
                rot: rand(0, Math.PI * 2),
                size: rand(0.15, 0.23),
            });
        }
    }

    // MARK: Update

    update(dt: number, now: number) {
        // Advance pending scheduled callbacks (greeting beats, emotes).
        if (this.pending.length) {
            const due = this.pending.filter((s) => s.at <= now);
            if (due.length) {
                this.pending = this.pending.filter((s) => s.at > now);
                for (const s of due) s.fn();
            }
        }

        // Advance tweens.
        for (const [key, tw] of [...this.tweens]) {
            const k = tw.keys[tw.keyIndex];
            const elapsed = now * 1000 - tw.startTime;
            const p = clamp(elapsed / k.duration, 0, 1);
            this.setProperty(key, tw.from + (k.target - tw.from) * k.ease(p));

            if (p >= 1) {
                tw.from = k.target;
                tw.keyIndex += 1;
                tw.startTime = now * 1000;

                if (tw.keyIndex >= tw.keys.length) {
                    this.tweens.delete(key);
                    this.locks.delete(key);
                    tw.onComplete?.();
                } else {
                    this.tweens.set(key, tw);
                }
            }
        }

        // Look targets.
        this.tgYaw = this.lookX * 0.62;
        this.tgPitch = this.lookY * 0.5;
        this.tgTilt = 0;

        // Searching scans side to side instead of holding still.
        if (this.searching && !this.locks.has("yaw")) {
            this.tgYaw = Math.sin(now * 1.6) * 0.5;
        }

        // Listening bobs to an inner tempo rather than tracking the pointer.
        if (this.listening && !this.searching) {
            if (!this.locks.has("tilt")) {
                this.tgTilt = Math.sin(now * 2.2) * 0.13;
            }
            if (!this.locks.has("yaw")) {
                this.tgYaw = Math.sin(now * 2.2) * 0.18;
            }
        }

        // Notes drift up on their own cadence while listening.
        if (this.listening && now - this.lastNote > 1.1) {
            this.emit("note", 1);
            this.lastNote = now;
        }

        // Body sway during the greeting wave.
        if (now > this.waveStart && now < this.waveUntil) {
            const wt = now - this.waveStart;
            this.tgTilt = -0.06 + Math.sin(2 * Math.PI * 1.2 * wt) * 0.07;
        }

        // Dizzy wobble on top of the roll.
        if (now < this.dizzyUntil && !this.locks.has("tilt")) {
            this.tgTilt = Math.sin(now * 9) * 0.25;
        }

        if (!this.locks.has("oy")) {
            this.oy += (0 - this.oy) * (1 - Math.pow(0.0008, dt));
        }

        this.tgSy = 1;
        this.tgSx = 1;

        const kLook = 1 - Math.pow(0.0025, dt);
        const kGen = 1 - Math.pow(0.0008, dt);

        if (!this.locks.has("yaw")) this.yaw += (this.tgYaw - this.yaw) * kLook;
        if (!this.locks.has("pitch")) {
            this.pitch += (this.tgPitch - this.pitch) * kLook;
        }
        if (!this.locks.has("tilt")) {
            this.tilt += (this.tgTilt - this.tilt) * kGen;
        }
        if (!this.locks.has("sy")) this.sy += (this.tgSy - this.sy) * kGen;
        if (!this.locks.has("sx")) this.sx += (this.tgSx - this.sx) * kGen;
        if (!this.locks.has("es")) this.es += (1 - this.es) * kGen;

        // Tint eases toward the state target.
        const tintK = 1 - Math.pow(0.002, dt);
        this.tint += (this.tintTarget - this.tint) * tintK;
        this.tintCol = mix3(this.tintCol, this.tintColor, tintK);

        // Idle blinking.
        if (now > this.nextBlink) {
            this.blink();
            this.animAt(0.23, () => this.blink(), false);
            this.nextBlink = now + 2.2 + Math.random() * 3.2;
        }

        // Expire eye overrides.
        if (this.eyeOverride && now > this.eyeOverrideUntil) {
            this.eyeOverride = null;
        }

        // Age particles.
        for (const p of this.particles) p.age += dt;
        this.particles = this.particles.filter((p) => p.age < p.life);
    }

    // MARK: Draw

    /**
     * Draw one frame. The body is sized from `bodyHeight` rather than from the
     * canvas width, so the canvas box can hug the character instead of carrying
     * a fixed 47% of transparent padding around it. Callers scale this for the
     * pop-in and tuck-out.
     */
    draw(ctx: CanvasRenderingContext2D, w: number, h: number, now: number) {
        // Body height is 1.76R (ry = 0.88R), so solve for R from the target.
        const R = (this.bodyHeight / 1.76) * this.scale;
        const rx = R * 1.14;
        const ry = R * 0.88;

        const cx = w / 2 + this.ox * R;
        const cy = h / 2 + this.oy * R + R * 0.06;

        this.drawHandsBehind(ctx, cx, cy, rx, ry, R, now);

        ctx.save();
        ctx.translate(cx, cy);
        if (this.tilt !== 0) ctx.rotate(this.tilt);
        ctx.scale(this.sx, this.sy);

        const body = mochiPath(rx, ry);

        this.drawBody(ctx, body, R, rx, ry);

        const blushVal = Math.max(this.blush, this.tint * 0.5);
        if (blushVal > 0.01) this.drawBlush(ctx, body, rx, ry, R, blushVal);

        this.drawEyes(ctx, body, R, rx, ry, now);

        ctx.restore();

        this.drawBadge(ctx, cx, cy, R);
        this.drawParticles(ctx, cx, cy, R);
    }

    /** Body scale, driven by the island's expand/collapse transition. */
    scale = 1;
    /** Body opacity, driven by the island's expand/collapse transition. */
    opacity = 1;
    /**
     * Body height in canvas units at scale 1. The canvas box is sized to fit
     * this, so the character fills its element instead of floating inside a
     * mostly-empty one.
     */
    bodyHeight = 96;

    private eyeOverride: EyeShape | null = null;
    private eyeOverrideUntil = 0;
    private pending: { at: number; fn: () => void }[] = [];
    private badgeToken = 0;
    private tintTarget = 0;
    private tintColor: RGB = IDLE_COLOR;
    private tintCol: RGB = IDLE_COLOR;
    private searching = false;
    private listening = false;
    private lastNote = 0;
    // private slapTimes: number[] = [];
    private dizzyUntil = 0;

    private animAt(delaySeconds: number, fn: () => void, dedupe = true) {
        const at = performance.now() / 1000 + delaySeconds;
        if (dedupe && this.pending.some((s) => Math.abs(s.at - at) < 1e-6)) {
            return;
        }
        this.pending.push({ at, fn });
    }

    private anim(key: string, keys: TweenKey[], onComplete?: () => void) {
        this.tweens.set(key, {
            property: key,
            keys,
            keyIndex: 0,
            from: this.getProperty(key),
            startTime: performance.now(),
            onComplete,
        });
        this.locks.add(key);
    }

    private setProperty(key: string, value: number) {
        const self = this as unknown as Record<PropertyKey, number>;
        if (key in self) self[key as PropertyKey] = value;
    }

    private getProperty(key: string): number {
        const self = this as unknown as Record<string, number>;
        return typeof self[key] === "number" ? self[key] : 0;
    }

    private drawBody(
        ctx: CanvasRenderingContext2D,
        path: Path2D,
        R: number,
        rx: number,
        ry: number
    ) {
        const grad = ctx.createLinearGradient(
            rx * 0.7,
            -ry * 0.85,
            -rx * 0.8,
            ry * 0.9
        );
        grad.addColorStop(0, css(MOCHI.baseTop));
        grad.addColorStop(1, css(MOCHI.baseBottom));
        ctx.fillStyle = grad;
        ctx.fill(path);

        // State tint, fading upward.
        if (this.tint > 0.01) {
            const tg = ctx.createLinearGradient(0, ry, 0, -ry);
            tg.addColorStop(0, css(this.tintCol, 0.72 * this.tint));
            tg.addColorStop(1, css(this.tintCol, 0));
            ctx.fillStyle = tg;
            ctx.fill(path);
        }

        // Shadow rim.
        ctx.save();
        ctx.clip(path);
        const rim = ctx.createRadialGradient(0, 0, R * 0.15, 0, 0, R * 1.25);
        rim.addColorStop(0, "rgba(0,0,0,0)");
        rim.addColorStop(0.6, "rgba(0,0,0,0)");
        rim.addColorStop(1, "rgba(0,0,0,0.2)");
        ctx.fillStyle = rim;
        ctx.fillRect(-R * 2, -R * 2, R * 4, R * 4);
        ctx.restore();

        // Highlight.
        ctx.save();
        ctx.clip(path);
        const hl = ctx.createRadialGradient(
            rx * 0.34,
            -ry * 0.46,
            0,
            rx * 0.34,
            -ry * 0.46,
            R * 0.42
        );
        hl.addColorStop(0, "rgba(255,255,255,0.55)");
        hl.addColorStop(1, "rgba(255,255,255,0)");
        ctx.fillStyle = hl;
        ctx.fillRect(-R * 2, -R * 2, R * 4, R * 4);
        ctx.restore();
    }

    private drawHandsBehind(
        ctx: CanvasRenderingContext2D,
        cx: number,
        cy: number,
        rx: number,
        ry: number,
        R: number,
        now: number
    ) {
        if (this.hands <= 0.01) return;
        // Only draw when Mochi is large enough for hands to read.
        if (R <= 14) return;

        const bodyH = 2 * ry;
        const hew = 0.3 * ry * this.hands;
        const heh = 0.26 * ry * this.hands;

        const hwB = rx * this.sx;
        const hhB = ry * this.sy;

        const isWaving = this.waveStart > 0 && now >= this.waveStart && now < this.waveUntil;

        for (const sd of [-1, 1]) {
            let localX: number;
            let localY: number;
            let handRot = 0;

            if (sd > 0 && isWaving) {
                const wt = now - this.waveStart;
                const rise = Math.min(1, wt / 0.18);
                const riseEased = 1 - Math.pow(1 - rise, 3);

                const restX = hwB * 1.08;
                const restY = hhB * 0.7;
                const oscX = Math.cos(13 * wt) * 0.06 * bodyH;
                const oscY = -Math.sin(13 * wt) * 0.14 * bodyH;
                const waveX = hwB * 1.1 + oscX;
                const waveY = -hhB * 0.15 + oscY;

                localX = restX + (waveX - restX) * riseEased;
                localY = restY + (waveY - restY) * riseEased;
                handRot = (-0.5 + Math.sin(13 * wt) * 0.35) * riseEased;
            } else if (sd < 0 && isWaving) {
                const wt = now - this.waveStart;
                localX = -hwB * 1.08;
                localY = hhB * 0.7 + Math.sin(6 * wt) * 0.04 * bodyH;
            } else {
                localX = sd * hwB * 1.08;
                localY = hhB * 0.7;
            }

            const cosT = Math.cos(this.tilt);
            const sinT = Math.sin(this.tilt);
            const worldX = cx + cosT * localX - sinT * localY;
            const worldY = cy + sinT * localX + cosT * localY;

            ctx.save();
            ctx.translate(worldX, worldY);
            if (handRot !== 0) ctx.rotate(handRot);

            const g = ctx.createLinearGradient(
                hew * 0.7,
                -heh * 0.85,
                -hew * 0.8,
                heh * 0.9
            );
            g.addColorStop(0, css(MOCHI.baseTop));
            g.addColorStop(1, css(MOCHI.baseBottom));

            ctx.beginPath();
            ctx.ellipse(0, 0, hew, heh, 0, 0, Math.PI * 2);
            ctx.fillStyle = g;
            ctx.fill();
            ctx.strokeStyle = "rgba(0,0,0,0.08)";
            ctx.lineWidth = 1;
            ctx.stroke();

            ctx.restore();
        }
    }

    private drawBlush(
        ctx: CanvasRenderingContext2D,
        path: Path2D,
        rx: number,
        ry: number,
        R: number,
        blush: number
    ) {
        ctx.save();
        ctx.clip(path);

        const yOffset = Math.sin(this.yaw) * rx * 0.8;
        for (const sd of [-1, 1]) {
            const bx = sd * rx * 0.55 + yOffset;
            const by = ry * 0.2;

            ctx.beginPath();
            ctx.ellipse(bx, by, R * 0.17, R * 0.1, 0, 0, Math.PI * 2);
            ctx.fillStyle = css(PINK, 0.5 * blush);
            ctx.fill();
        }

        ctx.restore();
    }

    private drawEyes(
        ctx: CanvasRenderingContext2D,
        path: Path2D,
        R: number,
        rx: number,
        ry: number,
        now: number
    ) {
        const shape = this.listening
            ? "closed"
            : this.eyeOverride ?? "pill";

        ctx.save();
        ctx.clip(path);

        for (const sd of [-1, 1]) {
            const eyeYaw = sd * MOCHI.eyeSp + this.yaw;
            let eyePitch = MOCHI.eyeP + this.pitch + this.roll;
            eyePitch =
                ((((eyePitch + Math.PI) % (Math.PI * 2)) + Math.PI * 2) %
                    (Math.PI * 2)) -
                Math.PI;

            const cp = Math.cos(eyePitch);
            // Eyes behind the head are hidden.
            if (Math.cos(eyeYaw) * cp <= 0.04) continue;

            const ex = Math.sin(eyeYaw) * cp * rx;
            const ey = -Math.sin(eyePitch) * ry;

            const fx = Math.max(0.18, Math.cos(eyeYaw));
            const fy = Math.max(0.18, cp);

            const ew = R * MOCHI.eyeW * this.es;
            const eh = R * MOCHI.eyeH * this.es;

            ctx.save();
            ctx.translate(ex, ey);
            ctx.scale(fx, fy);
            drawEyeShape(ctx, shape, ew, eh, this.open, sd, now);
            ctx.restore();
        }

        ctx.restore();
    }

    private drawBadge(
        ctx: CanvasRenderingContext2D,
        cx: number,
        cy: number,
        R: number
    ) {
        const badge = this.badge;
        if (!badge || this.badgeS <= 0.01) return;

        const bs = this.badgeS;
        const bx = cx - R * 0.72 * this.sx;
        const by = cy - R * 0.72 * this.sy;
        const now = performance.now() / 1000;

        ctx.save();
        ctx.translate(bx, by);
        ctx.scale(bs, bs);

        switch (badge.kind) {
            case "dots": {
                const pw = R * 0.72;
                const ph = R * 0.36;

                roundRect(ctx, -pw / 2, -ph / 2, pw, ph, ph / 2);
                ctx.fillStyle = css(badge.color);
                ctx.fill();

                for (let i = 0; i < 3; i++) {
                    const phase = fract(now * 2.4 - i * 0.22);
                    const dotR =
                        R * 0.055 * (1 + 0.4 * Math.max(0, Math.sin(phase * Math.PI * 2)));

                    ctx.beginPath();
                    ctx.arc((i - 1) * R * 0.18, 0, dotR, 0, Math.PI * 2);
                    ctx.fillStyle = "#ffffff";
                    ctx.fill();
                }
                break;
            }
            case "bang":
            case "question": {
                ctx.beginPath();
                ctx.arc(0, 0, R * 0.3, 0, Math.PI * 2);
                ctx.fillStyle = "#000000";
                ctx.fill();

                ctx.beginPath();
                ctx.arc(0, 0, R * 0.23, 0, Math.PI * 2);
                ctx.fillStyle = css(badge.color);
                ctx.fill();

                ctx.fillStyle = "#ffffff";
                ctx.font = `900 ${R * 0.32}px system-ui, sans-serif`;
                ctx.textAlign = "center";
                ctx.textBaseline = "middle";
                ctx.fillText(badge.kind === "bang" ? "!" : "?", 0, R * 0.02);
                break;
            }
            case "dot": {
                ctx.beginPath();
                ctx.arc(0, 0, R * 0.2, 0, Math.PI * 2);
                ctx.fillStyle = "#000000";
                ctx.fill();

                ctx.beginPath();
                ctx.arc(0, 0, R * 0.135, 0, Math.PI * 2);
                ctx.fillStyle = css(badge.color);
                ctx.fill();
                break;
            }
            case "note": {
                // Headphones, in the state's colour.
                const r = R * 0.26;
                ctx.strokeStyle = css(badge.color);
                ctx.lineWidth = R * 0.075;
                ctx.lineCap = "round";

                // Headband.
                ctx.beginPath();
                ctx.arc(0, r * 0.1, r, Math.PI, Math.PI * 2);
                ctx.stroke();

                // Ear cups.
                ctx.fillStyle = css(badge.color);
                ctx.beginPath();
                ctx.ellipse(-r * 0.98, r * 0.28, r * 0.3, r * 0.42, 0, 0, Math.PI * 2);
                ctx.fill();
                ctx.beginPath();
                ctx.ellipse(r * 0.98, r * 0.28, r * 0.3, r * 0.42, 0, 0, Math.PI * 2);
                ctx.fill();
                break;
            }
        }

        ctx.restore();
    }

    private drawParticles(
        ctx: CanvasRenderingContext2D,
        cx: number,
        cy: number,
        R: number
    ) {
        for (const p of this.particles) {
            if (p.age <= 0) continue;

            const k = p.age / p.life;
            const a = k < 0.2 ? k / 0.2 : 1 - (k - 0.2) / 0.8;
            const px = cx + (p.x + p.vx * p.age) * R * 1.3;
            const py = cy + (p.y + p.vy * p.age) * R * 1.3;
            const sz = R * p.size * (1 + k * 0.4);

            ctx.save();
            ctx.translate(px, py);
            ctx.globalAlpha = clamp(a, 0, 1);

            switch (p.type) {
                case "heart":
                    ctx.rotate(Math.sin(p.age * 6) * 0.3);
                    ctx.fillStyle = css(HEART);
                    heart(ctx, sz);
                    ctx.fill();
                    break;
                case "star":
                    ctx.rotate(p.rot + p.age * 2);
                    ctx.fillStyle = css(STAR);
                    star(ctx, sz, sz * 0.45);
                    ctx.fill();
                    break;
                case "spark":
                    ctx.rotate(p.rot);
                    ctx.fillStyle = "#ffffff";
                    star(ctx, sz * 0.8, sz * 0.18);
                    ctx.fill();
                    break;
                case "sweat": {
                    ctx.beginPath();
                    ctx.moveTo(0, -sz);
                    ctx.quadraticCurveTo(sz * 0.8, sz * 0.2, 0, sz * 0.6);
                    ctx.quadraticCurveTo(-sz * 0.8, sz * 0.2, 0, -sz);
                    ctx.fillStyle = css(SWEAT);
                    ctx.fill();
                    break;
                }
                case "z":
                    ctx.fillStyle = css(Z_INK);
                    ctx.font = `bold ${sz * 1.9}px system-ui, sans-serif`;
                    ctx.textAlign = "center";
                    ctx.textBaseline = "middle";
                    ctx.fillText("z", 0, 0);
                    break;
                case "note": {
                    // A beamed pair of eighth notes, swaying as it rises.
                    ctx.rotate(Math.sin(p.age * 2.4) * 0.16);
                    ctx.strokeStyle = css(NOTE_COLOR);
                    ctx.fillStyle = css(NOTE_COLOR);
                    ctx.lineCap = "round";

                    const headR = sz * 0.3;
                    const headH = sz * 0.22;
                    const lx = -sz * 0.34;
                    const ly = sz * 0.52;
                    const rx2 = sz * 0.4;
                    const ry2 = sz * 0.22;
                    const top = -sz * 0.95;

                    // Stems.
                    ctx.lineWidth = sz * 0.15;
                    ctx.beginPath();
                    ctx.moveTo(lx, ly);
                    ctx.lineTo(lx, top);
                    ctx.moveTo(rx2, ry2);
                    ctx.lineTo(rx2, top);
                    ctx.stroke();

                    // Beam.
                    ctx.lineWidth = sz * 0.2;
                    ctx.beginPath();
                    ctx.moveTo(lx, top);
                    ctx.lineTo(rx2, top);
                    ctx.stroke();

                    // Note heads, tilted like real engraving.
                    ctx.beginPath();
                    ctx.ellipse(lx, ly, headR, headH, -0.35, 0, Math.PI * 2);
                    ctx.fill();
                    ctx.beginPath();
                    ctx.ellipse(rx2, ry2, headR, headH, -0.35, 0, Math.PI * 2);
                    ctx.fill();
                    break;
                }
            }

            ctx.restore();
        }
    }
}

// MARK: - Free functions

function drawEyeShape(
    ctx: CanvasRenderingContext2D,
    shape: EyeShape,
    w: number,
    h: number,
    open: number,
    sd: number,
    now: number
) {
    const ink = css(MOCHI.ink);

    switch (shape) {
        case "wide":
            drawEyeShape(ctx, "pill", w * 1.16, h * 1.12, open, sd, now);
            break;

        case "pill": {
            const hh = Math.max(h * open, w * 0.3);
            roundRect(ctx, -w / 2, -hh / 2, w, hh, Math.min(w / 2, hh / 2));
            ctx.fillStyle = ink;
            ctx.fill();
            break;
        }

        case "dot":
            ctx.beginPath();
            ctx.arc(0, 0, w * 0.45, 0, Math.PI * 2);
            ctx.fillStyle = ink;
            ctx.fill();
            break;

        case "line":
            ctx.save();
            ctx.rotate(-sd * 0.2);
            roundRect(ctx, -w * 0.78, -w * 0.21, w * 1.56, w * 0.42, w * 0.21);
            ctx.fillStyle = ink;
            ctx.fill();
            ctx.restore();
            break;

        case "flat":
            roundRect(ctx, -w * 0.72, -h * 0.2, w * 1.44, h * 0.4, w * 0.2);
            ctx.fillStyle = ink;
            ctx.fill();
            break;

        case "happy":
            ctx.beginPath();
            ctx.arc(0, h * 0.18, w * 0.82, deg(180 + 12), deg(180 - 12));
            ctx.strokeStyle = ink;
            ctx.lineWidth = w * 0.5;
            ctx.lineCap = "round";
            ctx.stroke();
            break;

        case "closed":
            ctx.beginPath();
            ctx.arc(0, -h * 0.08, w * 0.78, deg(15), deg(165));
            ctx.strokeStyle = ink;
            ctx.lineWidth = w * 0.36;
            ctx.lineCap = "round";
            ctx.stroke();
            break;

        case "spiral": {
            ctx.beginPath();
            let a = 0;
            let first = true;
            while (a < 4.4 * Math.PI) {
                const r = w * 0.06 + a * w * 0.058;
                const aa = a + now * 9 * sd;
                const px = Math.cos(aa) * r;
                const py = Math.sin(aa) * r;
                if (first) {
                    ctx.moveTo(px, py);
                    first = false;
                } else {
                    ctx.lineTo(px, py);
                }
                a += 0.2;
            }
            ctx.strokeStyle = ink;
            ctx.lineWidth = w * 0.22;
            ctx.lineCap = "round";
            ctx.stroke();
            break;
        }

        case "heart":
            ctx.fillStyle = css(HEART);
            heart(ctx, w * 1.2);
            ctx.fill();
            break;

        case "star":
            ctx.save();
            ctx.rotate(now * 1.5 * sd);
            ctx.fillStyle = css(STAR);
            star(ctx, w * 1.05, w * 0.46);
            ctx.fill();
            ctx.restore();
            break;

        case "tired":
            roundRect(ctx, -w / 2, -h * 0.02, w, h * 0.38, w / 2);
            ctx.fillStyle = ink;
            ctx.fill();
            roundRect(ctx, -w * 0.62, -h * 0.1, w * 1.24, w * 0.22, w * 0.11);
            ctx.fill();
            break;

        case "wink":
            if (sd < 0) {
                const hh = Math.max(h * open, w * 0.3);
                roundRect(ctx, -w / 2, -hh / 2, w, hh, Math.min(w / 2, hh / 2));
                ctx.fillStyle = ink;
                ctx.fill();
            } else {
                ctx.beginPath();
                ctx.arc(0, h * 0.18, w * 0.82, deg(180 + 12), deg(180 - 12));
                ctx.strokeStyle = ink;
                ctx.lineWidth = w * 0.5;
                ctx.lineCap = "round";
                ctx.stroke();
            }
            break;
    }
}

/**
 * Mochi's silhouette: a superellipse sampled as a polygon. Exponent 2/2.7 gives
 * the rounded-square blob the character is built from.
 */
function mochiPath(rx: number, ry: number): Path2D {
    const n = 72;
    const expN = 2.0 / 2.7;
    const path = new Path2D();

    for (let i = 0; i <= n; i++) {
        const a = (i / n) * Math.PI * 2;
        const ca = Math.cos(a);
        const sa = Math.sin(a);
        const px = rx * (ca >= 0 ? Math.pow(ca, expN) : -Math.pow(-ca, expN));
        const py = ry * (sa >= 0 ? Math.pow(sa, expN) : -Math.pow(-sa, expN));

        if (i === 0) path.moveTo(px, py);
        else path.lineTo(px, py);
    }

    path.closePath();
    return path;
}

function heart(ctx: CanvasRenderingContext2D, s: number) {
    ctx.beginPath();
    ctx.moveTo(0, s * 0.38);
    ctx.bezierCurveTo(-s * 1.05, -s * 0.15, -s * 0.5, -s * 0.95, 0, -s * 0.38);
    ctx.bezierCurveTo(s * 0.5, -s * 0.95, s * 1.05, -s * 0.15, 0, s * 0.38);
    ctx.closePath();
}

function star(ctx: CanvasRenderingContext2D, ro: number, ri: number) {
    ctx.beginPath();
    for (let i = 0; i < 10; i++) {
        const r = i % 2 === 0 ? ro : ri;
        const a = -Math.PI / 2 + i * (Math.PI / 5);
        const x = Math.cos(a) * r;
        const y = Math.sin(a) * r;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    }
    ctx.closePath();
}

function roundRect(
    ctx: CanvasRenderingContext2D,
    x: number,
    y: number,
    w: number,
    h: number,
    r: number
) {
    const rr = Math.max(0, Math.min(r, w / 2, h / 2));
    ctx.beginPath();
    ctx.moveTo(x + rr, y);
    ctx.arcTo(x + w, y, x + w, y + h, rr);
    ctx.arcTo(x + w, y + h, x, y + h, rr);
    ctx.arcTo(x, y + h, x, y, rr);
    ctx.arcTo(x, y, x + w, y, rr);
    ctx.closePath();
}

function emoteEyeShape(e: Emote): EyeShape {
    switch (e) {
        case "love":
            return "heart";
        case "surprised":
            return "dot";
        case "proud":
            return "star";
        case "wink":
            return "wink";
        case "yawn":
            return "tired";
        case "happy":
            return "happy";
        case "annoyed":
            return "line";
    }
}

function badgeKeyOf(b: Badge | null): string {
    if (!b) return "none";
    return `${b.kind}-${b.color.join(",")}`;
}

const fract = (v: number) => v - Math.floor(v);
const deg = (d: number) => (d * Math.PI) / 180;

export const MOCHI_IDLE: RGB = [0.902, 0.914, 0.933];
export const MOCHI_WORKING: RGB = [0.231, 0.62, 1];
export const MOCHI_THINKING: RGB = [0.545, 0.361, 0.965];
