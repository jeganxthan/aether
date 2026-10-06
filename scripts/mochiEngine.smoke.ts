// Headless smoke test for the Mochi engine: runs the full timeline with a stub
// 2D context so drawing maths and the tween system are exercised without a DOM.
//
//   npx tsx src/ui/island/mochi/mochiEngine.smoke.ts
//
// (Run manually; not part of the build.)

import { MochiEngine } from "../src/ui/island/mochi/mochiEngine";

const ops: Record<string, number> = {};
let current = "";

const stubCtx = new Proxy(
    {},
    {
        get(_t, prop: string) {
            if (prop === "canvas") return {};
            if (prop === "measureText") return () => ({ width: 10 });

            // Gradients need to be chainable and record their colour stops.
            if (prop === "createLinearGradient" || prop === "createRadialGradient") {
                return (..._args: number[]) => ({
                    addColorStop: (offset: number, color: string) => {
                        ops[`${current}:stop${offset}`] = color.length;
                    },
                });
            }

            return (...args: unknown[]) => {
                ops[prop] = (ops[prop] ?? 0) + 1;
                void args;
            };
        },
        set(_t, prop: string, value: unknown) {
            if (typeof value === "string") {
                ops[`${current}:${prop}`] = value.length;
            }
            return true;
        },
    }
) as unknown as CanvasRenderingContext2D;

globalThis.Path2D = class {
    moveTo() {}
    lineTo() {}
    closePath() {}
    arc() {}
    rect() {}
    quadraticCurveTo() {}
    bezierCurveTo() {}
} as unknown as typeof Path2D;

globalThis.performance = globalThis.performance ?? ({} as Performance);
if (typeof globalThis.performance.now !== "function") {
    globalThis.performance.now = () => 0;
}

const engine = new MochiEngine();
engine.setEmote("love");
engine.greet();

// Simulate 4s at 60fps, snapping expanded/collapsed midway to exercise both.
const dt = 1 / 60;
let t = 0;

for (let frame = 0; frame < 240; frame++) {
    current = "frame";
    engine.scale = frame < 120 ? 1 : 0.34;
    engine.opacity = frame < 120 ? 1 : 0.9;
    engine.lookX = Math.sin(t * 2) * 0.6;
    engine.lookY = Math.cos(t * 1.5) * 0.4;

    engine.update(dt, t);
    engine.draw(stubCtx, 68, 68, t);

    t += dt;
}

const drawn = Object.keys(ops).filter((k) => !k.includes(":"));
// clearRect is the canvas component's job, not the engine's.
const required = [
    "fill",
    "stroke",
    "save",
    "restore",
    "translate",
    "clip",
    "ellipse",
    "bezierCurveTo",
];

const missing = required.filter((op) => !drawn.includes(op));

console.log("frames drawn: 240");
console.log("distinct draw ops:", drawn.sort().join(", "));
console.log("fill calls:", ops.fill ?? 0, "| stroke calls:", ops.stroke ?? 0);

if (missing.length) {
    console.error("MISSING draw ops:", missing.join(", "));
    process.exit(1);
}

console.log("OK — no exceptions, all required draw ops exercised");
