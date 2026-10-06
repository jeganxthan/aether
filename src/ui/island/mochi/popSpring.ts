/**
 * The pop-in spring that carries Mochi's own overshoot on top of the island's
 * CSS size change.
 *
 * This lives outside the component so it can be stepped deterministically in a
 * test. The bug it replaces was subtle: the spring was re-kicked to its initial
 * value on *every* animation frame while the island was open, so it could never
 * integrate toward its rest value and Mochi was frozen at 57.8% scale with the
 * greeting permanently disarmed. React now calls `kick()` once per transition
 * and the loop only ever calls `step()`.
 */
export class PopSpring {
    /** Where the character sits when fully open. */
    readonly rest: number;

    /** Value the spring is thrown to when the island opens. */
    readonly from: number;

    /** Undamped angular frequency, in radians per second. */
    readonly omega: number;

    /** Damping ratio. Below 1, so it overshoots. */
    readonly zeta: number;

    private current: number;
    private velocity = 0;

    constructor(options?: {
        rest?: number;
        from?: number;
        /** Seconds for one undamped oscillation. */
        response?: number;
        damping?: number;
    }) {
        const {
            rest = 1,
            from = 0.55,
            // Response 0.42s / damping 0.55 — snappier than the island's own
            // spring so the character lands first and the window catches up.
            response = 0.42,
            damping = 0.55,
        } = options ?? {};

        this.rest = rest;
        this.from = from;
        this.omega = (2 * Math.PI) / response;
        this.zeta = damping;
        this.current = rest;
    }

    /** Throw the spring toward `from`, from a standstill. Idempotent-safe. */
    kick(): void {
        this.current = this.from;
        this.velocity = 0;
    }

    /** Land immediately on the rest value with no motion. */
    snapToRest(): void {
        this.current = this.rest;
        this.velocity = 0;
    }

    get value(): number {
        return this.current;
    }

    /**
     * True once the character is far enough up for the greeting to read as a
     * wave rather than a double-take.
     */
    get readyToGreet(): boolean {
        return this.current > 0.85;
    }

    /** True once the spring has effectively stopped moving. */
    get settled(): boolean {
        return Math.abs(this.current - this.rest) < 0.001 && Math.abs(this.velocity) < 0.001;
    }

    /**
     * Advance by `dt` seconds. The timestep is clamped so a long frame (or a
     * backgrounded tab) cannot blow the integrator up.
     */
    step(dt: number): number {
        const step = Math.min(0.05, Math.max(0, dt));

        const acc =
            this.omega * this.omega * (this.rest - this.current) -
            2 * this.zeta * this.omega * this.velocity;

        this.velocity += acc * step;
        this.current += this.velocity * step;

        return this.current;
    }
}
