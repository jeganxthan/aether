import { PopSpring } from "../src/ui/island/mochi/popSpring";

/**
 * Regression coverage for the Mochi pop-in spring.
 *
 * The original bug was that the spring was re-kicked to its initial value on
 * every animation frame while the island was open. It never escaped that value:
 * it sat at 57.8% scale and the greeting was permanently disarmed, which is what
 * read as Mochi flickering. These assertions pin the two behaviours that fix it.
 */

const FRAME = 1 / 60;

function advance(spring: PopSpring, frames: number): void {
    for (let i = 0; i < frames; i += 1) {
        spring.step(FRAME);
    }
}

const failures: string[] = [];

function check(label: string, condition: boolean): void {
    if (condition) {
        console.log(`  ok    ${label}`);
    } else {
        failures.push(label);
        console.log(`  FAIL  ${label}`);
    }
}

console.log("PopSpring");

// 1. A kicked spring must move away from its starting value.
{
    const spring = new PopSpring();
    spring.kick();

    check("starts at the kicked value", Math.abs(spring.value - 0.55) < 1e-9);

    advance(spring, 1);
    const afterOneFrame = spring.value;

    check(
        "moves on the very next frame",
        Math.abs(afterOneFrame - 0.55) > 1e-6,
    );
}

// 2. The real regression: if the old code re-pinned the spring each frame, it
//    would stay near 0.55 forever and never greet. A single kick per transition
//    must let it reach the greeting threshold and settle at rest.
{
    const spring = new PopSpring();
    spring.kick();

    let greeted = false;
    let framesToGreet = -1;
    for (let i = 0; i < 240; i += 1) {
        const value = spring.step(FRAME);
        if (!greeted && value > 0.85) {
            greeted = true;
            framesToGreet = i;
        }
    }

    check("reaches the greeting threshold", greeted);
    check("greets promptly (< 1s)", greeted && framesToGreet >= 0 && framesToGreet < 60);
    check(
        "settles at rest scale 1",
        Math.abs(spring.value - 1) < 1e-3,
    );
    check("reports settled", spring.settled);
}

// 3. The old code froze it at 0.5780; assert we are nowhere near that value
//    after a full expansion.
{
    const spring = new PopSpring();
    spring.kick();
    advance(spring, 240);

    check(
        "does not stay stuck at the old 0.578 freeze",
        Math.abs(spring.value - 0.578) > 0.4,
    );
}

// 4. Collapse must land immediately, with no residual motion.
{
    const spring = new PopSpring();
    spring.kick();
    advance(spring, 10);
    spring.snapToRest();

    check("snaps to rest on collapse", spring.value === 1);
    check("has no residual velocity", spring.settled);
}

// 5. Re-opening must kick again, so a second expansion animates rather than
//    sitting still at rest.
{
    const spring = new PopSpring();
    spring.kick();
    advance(spring, 240);
    spring.snapToRest();
    spring.kick();

    check("re-kicks on a later expansion", spring.value === 0.55);
    advance(spring, 240);
    check("second expansion also settles at 1", Math.abs(spring.value - 1) < 1e-3);
}

// 6. Overshoot is intentional: it is what makes the pop read as springy.
{
    const spring = new PopSpring();
    spring.kick();

    let peak = 0;
    for (let i = 0; i < 240; i += 1) {
        peak = Math.max(peak, spring.step(FRAME));
    }

    check("overshoots past rest", peak > 1.0, );
    check("overshoot is bounded (< 1.15)", peak < 1.15);
}

// 7. A huge timestep (backgrounded tab) must not blow the integrator up.
{
    const spring = new PopSpring();
    spring.kick();
    spring.step(5);

    check("survives a 5s frame", Number.isFinite(spring.value));
    advance(spring, 240);
    check("recovers after a huge frame", Math.abs(spring.value - 1) < 1e-3);
}

if (failures.length > 0) {
    console.error(`\nPopSpring: ${failures.length} failing assertion(s)`);
    for (const failure of failures) {
        console.error(`  - ${failure}`);
    }
    process.exit(1);
}

console.log("\nPopSpring: all assertions passed");
