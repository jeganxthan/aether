import { useEffect, useRef } from "react";
import { MochiEngine } from "./mochiEngine";
import { PopSpring } from "./popSpring";

/**
 * The canvas is always EXPANDED_W x EXPANDED_H. Only a uniform CSS scale
 * changes between states, so the backing store is allocated once and never
 * reallocated mid-animation.
 *
 * The box is wider than the body because Mochi's hands reach past its sides
 * while waving — sizing to the body alone would clip them.
 */
const EXPANDED_W = 140;
const EXPANDED_H = 104;

/** Body height at scale 1, matched to the box above. */
const EXPANDED_BODY = 80;

/** Scale of the whole character in the collapsed strip. */
const RESTING_SCALE = 0.24;

/** Matches the island's own transition so the two settle together. */
const SIZE_EASING = "cubic-bezier(0.16,1,0.3,1)";
const SIZE_DURATION = 500;

interface MochiCanvasProps {
    /** True while the island is expanded. Drives the pop-in / tuck-out. */
    expanded: boolean;
    /** Optional emote to trigger when entering expanded state. */
    emote?: "love" | "surprised" | "proud" | "wink" | "yawn" | "happy" | "annoyed";
    /** Puts Mochi into its blue "searching" state while a scan runs. */
    searching?: boolean;
    /** Puts Mochi into its green "listening" state while music plays. */
    listening?: boolean;
}

/**
 * The Mochi character, drawn on a canvas and animated in/out by the island's
 * expand state. The engine advances on its own rAF loop; React only pushes the
 * `expanded` transition in and tears the loop down on unmount.
 */
export default function MochiCanvas({ expanded, emote, searching = false, listening = false }: MochiCanvasProps) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const engineRef = useRef<MochiEngine | null>(null);
    const greetedRef = useRef(false);
    const emoteTriggeredRef = useRef(false);
    const expandedRef = useRef(expanded);
    const popSpringRef = useRef<PopSpring | null>(null);

    if (engineRef.current === null) {
        engineRef.current = new MochiEngine();
    }
    if (popSpringRef.current === null) {
        popSpringRef.current = new PopSpring();
    }

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
        engineRef.current?.setListening(listening && expanded);
    }, [listening, expanded]);

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const engine = engineRef.current!;
        const ctx = canvas.getContext("2d");
        if (!ctx) return;

        const dpr = window.devicePixelRatio || 1;
        canvas.width = Math.round(EXPANDED_W * dpr);
        canvas.height = Math.round(EXPANDED_H * dpr);
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

        const pointer = { x: 0, y: 0, seen: false };
        const onMove = (e: PointerEvent) => {
            pointer.x = e.clientX;
            pointer.y = e.clientY;
            pointer.seen = true;
        };
        window.addEventListener("pointermove", onMove);

        let raf = 0;
        let last = performance.now() / 1000;

        // The look-at target is the canvas centre. It only moves when the island
        // is resized or the window itself moves, so it is measured on demand
        // instead of every frame — reading it per frame forces a synchronous
        // layout on each step of the rAF loop.
        let centerX = 0;
        let centerY = 0;
        const remeasure = () => {
            const rect = canvas.getBoundingClientRect();
            centerX = rect.left + rect.width / 2;
            centerY = rect.top + rect.height / 2;
        };
        remeasure();
        window.addEventListener("resize", remeasure);

        const spring = popSpringRef.current!;
        let fade = 1;

        const step = () => {
            const now = performance.now() / 1000;
            const dt = Math.min(0.05, now - last);
            last = now;

            const isExpanded = expandedRef.current;

            const pop = spring.step(dt);

            const fadeK = 1 - Math.pow(0.0005, dt);
            fade += ((isExpanded ? 1 : 0.92) - fade) * fadeK;

            engine.bodyHeight = EXPANDED_BODY;
            engine.scale = Math.max(0.05, pop);
            engine.opacity = fade;

            // Look-at follows the pointer, once it has actually moved inside the
            // window. Before that the eyes sit centred.
            if (isExpanded && pointer.seen) {
                engine.lookX = Math.tanh((pointer.x - centerX) / 120);
                engine.lookY = -Math.tanh((pointer.y - centerY) / 90);
            }

            engine.update(dt, now);

            // Greet once the body is mostly up, not on the first frame —
            // greeting while still scaling reads as a double-take.
            if (isExpanded && !greetedRef.current && spring.readyToGreet) {
                greetedRef.current = true;
                if (emote && !emoteTriggeredRef.current) {
                    emoteTriggeredRef.current = true;
                    engineRef.current?.setEmote(emote, 1.6);
                } else {
                    engine.greet();
                }
            }

            ctx.clearRect(0, 0, EXPANDED_W, EXPANDED_H);
            ctx.save();
            ctx.globalAlpha = engine.opacity;
            engine.draw(ctx, EXPANDED_W, EXPANDED_H, now);
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

    const k = expanded ? 1 : RESTING_SCALE;

    return (
        <div
            className="relative shrink-0 pointer-events-auto cursor-pointer transition-all"
            onClick={() => {
                if (expandedRef.current) engineRef.current?.poke();
            }}
            style={{
                width: EXPANDED_W * k,
                height: EXPANDED_H * k,
                transitionDuration: `${SIZE_DURATION}ms`,
                transitionTimingFunction: SIZE_EASING,
            }}
        >
            <canvas
                ref={canvasRef}
                className="absolute left-1/2 top-1/2 pointer-events-none select-none"
                style={{
                    width: EXPANDED_W,
                    height: EXPANDED_H,
                    transform: `translate(-50%, -50%) scale(${k})`,
                    transition: `transform ${SIZE_DURATION}ms ${SIZE_EASING}`,
                }}
            />
        </div>
    );
}
