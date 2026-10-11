import { useEffect, useRef } from "react";
import { MochiEngine, type NoticeKind } from "./mochiEngine";
import { PopSpring } from "./popSpring";
import { Sound } from "./sound";

/**
 * The character is drawn at the size it is actually shown. The canvas backing
 * store is resized to the display box (x devicePixelRatio) every time the box
 * changes, so Mochi is never rasterised big and then squashed down by a CSS
 * transform — that 4x downscale is what made the resting character look
 * pixel-broken.
 *
 * The box is wider than the body because Mochi's hands reach past its sides
 * while waving — sizing to the body alone would clip them.
 */
const EXPANDED_W = 154;
const EXPANDED_H = 114;

/** Body height at scale 1, matched to the box above. */
const EXPANDED_BODY = 88;

/** Scale of the whole character in the collapsed strip. */
const RESTING_SCALE = 0.2;

/** Scale of the whole character while the music row is on screen. */
const MUSIC_SCALE = 0.55;

/** Scale of the whole character beside the launcher input. */
const LAUNCHER_SCALE = 0.45;

interface MochiCanvasProps {
    /** True while the island is expanded. Drives the pop-in / tuck-out. */
    expanded: boolean;
    /** Optional emote to trigger when entering expanded state. */
    emote?: "love" | "surprised" | "proud" | "wink" | "yawn" | "happy" | "annoyed";
    /** Puts Mochi into its blue "searching" state while a scan runs. */
    searching?: boolean;
    /** Puts Mochi into its green "listening" state while music plays. */
    listening?: boolean;
    /** Renders Mochi smaller beside the music row. */
    music?: boolean;
    /** Renders Mochi smaller beside the launcher input. */
    launcher?: boolean;
    /** Persistent notification look while a popup is up. */
    notice?: NoticeKind;
    /** Overrides the scale computed from the other props (notification popup). */
    scale?: number;
    /** Plays the greeting wave on expand. Off for the notification popup. */
    greet?: boolean;
}

/**
 * The Mochi character, drawn on a canvas and animated in/out by the island's
 * expand state. The engine advances on its own rAF loop and resizes the canvas
 * to whatever size the character is shown at, so it stays crisp at every scale.
 */
export default function MochiCanvas({ expanded, emote, searching = false, listening = false, music = false, launcher = false, notice = "normal", scale, greet = true }: MochiCanvasProps) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const boxRef = useRef<HTMLDivElement>(null);
    const engineRef = useRef<MochiEngine | null>(null);
    const greetedRef = useRef(false);
    const greetRef = useRef(greet);
    greetRef.current = greet;
    const emoteTriggeredRef = useRef(false);
    const expandedRef = useRef(expanded);
    const popSpringRef = useRef<PopSpring | null>(null);

    if (engineRef.current === null) {
        engineRef.current = new MochiEngine();
    }
    if (popSpringRef.current === null) {
        popSpringRef.current = new PopSpring();
    }

    const targetScale = scale ??
        (expanded
            ? launcher
                ? LAUNCHER_SCALE
                : music
                    ? MUSIC_SCALE
                    : 1
            : RESTING_SCALE);
    const targetScaleRef = useRef(targetScale);
    targetScaleRef.current = targetScale;

    /*
     * The box is sized by the render loop as the character eases between
     * scales. A stable inline style (same object every render) seeds the first
     * paint; React never rewrites it, so it cannot fight the loop.
     */
    const initialBoxStyle = useRef({
        width: EXPANDED_W * targetScale,
        height: EXPANDED_H * targetScale,
    }).current;

    // The pop-in spring is advanced by the render loop, so React only has to fire
    // the two transitions: kick it when the island opens, snap it shut when it
    // closes. Doing the kick inside the loop re-pinned the spring every frame and
    // it never escaped its initial value.
    useEffect(() => {
        const spring = popSpringRef.current!;
        expandedRef.current = expanded;

        if (expanded) {
            spring.kick();
        } else {
            spring.snapToRest();
            greetedRef.current = false;
            emoteTriggeredRef.current = false;
            engineRef.current?.interruptGreet();
        }
    }, [expanded]);

    useEffect(() => {
        engineRef.current?.setSearching(searching && expanded);
    }, [searching, expanded]);

    useEffect(() => {
        engineRef.current?.setListening(
            listening,
        );
    }, [listening]);

    useEffect(() => {
        engineRef.current?.setNoticeState(notice);
    }, [notice]);

    useEffect(() => {
        const engine = engineRef.current!;
        Sound.preload();
        engine.onSquash = () => Sound.play("click");
        engine.onGulp = () => Sound.play("gulp");
        return () => {
            engine.onSquash = null;
            engine.onGulp = null;
        };
    }, []);

    useEffect(() => {
        const canvas = canvasRef.current;
        const box = boxRef.current;
        if (!canvas || !box) return;

        const engine = engineRef.current!;
        const ctx = canvas.getContext("2d");
        if (!ctx) return;

        const dpr = Math.min(2, window.devicePixelRatio || 1);

        const pointer = { x: 0, y: 0, seen: false };
        const onMove = (e: PointerEvent) => {
            pointer.x = e.clientX;
            pointer.y = e.clientY;
            pointer.seen = true;
        };
        window.addEventListener("pointermove", onMove);

        let raf = 0;
        let last = performance.now() / 1000;

        // The look-at target is the character's centre. It only moves when the
        // island is resized or the window itself moves, so it is measured on
        // demand instead of every frame — reading it per frame forces a
        // synchronous layout on each step of the rAF loop.
        let centerX = 0;
        let centerY = 0;
        const remeasure = () => {
            const rect = box.getBoundingClientRect();
            centerX = rect.left + rect.width / 2;
            centerY = rect.top + rect.height / 2;
        };
        remeasure();
        window.addEventListener("resize", remeasure);

        const spring = popSpringRef.current!;
        let fade = 1;
        let k = targetScaleRef.current;
        let lastW = 0;
        let lastH = 0;

        const step = () => {
            const now = performance.now() / 1000;
            const dt = Math.min(0.05, now - last);
            last = now;

            const isExpanded = expandedRef.current;

            const pop = spring.step(dt);

            const fadeK = 1 - Math.pow(0.0005, dt);
            fade += ((isExpanded ? 1 : 0.92) - fade) * fadeK;

            // Ease the whole-character scale toward its target so the backing
            // store can be resized to the exact size on screen. Rasterising at
            // display size is what keeps the character crisp when small.
            k += (targetScaleRef.current - k) * (1 - Math.pow(0.0009, dt));

            const w = EXPANDED_W * k;
            const h = EXPANDED_H * k;
            const bw = Math.max(1, Math.round(w * dpr));
            const bh = Math.max(1, Math.round(h * dpr));
            if (bw !== lastW || bh !== lastH) {
                lastW = bw;
                lastH = bh;
                canvas.width = bw;
                canvas.height = bh;
                canvas.style.width = `${w}px`;
                canvas.style.height = `${h}px`;
                box.style.width = `${w}px`;
                box.style.height = `${h}px`;
            }
            ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

            engine.bodyHeight = EXPANDED_BODY * k;
            engine.scale = Math.max(0.05, pop);
            engine.opacity = fade;

            // Look-at follows the pointer, once it has actually moved inside the
            // window. This is deliberately not gated on `isExpanded`: the eyes
            // keep tracking while the island is collapsed, and hold their last
            // heading afterwards instead of snapping back to centre.
            if (pointer.seen) {
                engine.lookX = Math.tanh((pointer.x - centerX) / 120);
                engine.lookY = -Math.tanh((pointer.y - centerY) / 90);
            }

            engine.update(dt, now);

            // Greet once the body is mostly up, not on the first frame —
            // greeting while still scaling reads as a double-take.
            // In music mode the greeting is skipped: the character drops
            // straight into the listening state set by setListening.
            if (isExpanded && greetRef.current && !greetedRef.current && spring.readyToGreet) {
                greetedRef.current = true;
                if (music) {
                    /* no greeting — listening state only */
                } else if (emote && !emoteTriggeredRef.current) {
                    emoteTriggeredRef.current = true;
                    engineRef.current?.setEmote(emote, 1.6);
                } else {
                    engine.greet();
                }
            }

            ctx.clearRect(0, 0, w, h);
            ctx.save();
            ctx.globalAlpha = engine.opacity;
            engine.draw(ctx, w, h, now);
            ctx.restore();

            raf = requestAnimationFrame(step);
        };

        raf = requestAnimationFrame(step);

        return () => {
            cancelAnimationFrame(raf);
            window.removeEventListener("pointermove", onMove);
            window.removeEventListener("resize", remeasure);
        };
    }, []);

    return (
        <div
            ref={boxRef}
            className="relative shrink-0 pointer-events-auto cursor-pointer"
            onClick={() => {
                if (expandedRef.current) engineRef.current?.poke();
            }}
            style={initialBoxStyle}
        >
            <canvas
                ref={canvasRef}
                className="absolute left-0 top-0 block pointer-events-none select-none"
            />
        </div>
    );
}


