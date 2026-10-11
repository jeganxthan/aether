const FILES = {
    click: "mochi/click.wav",
    gulp: "mochi/gulp.wav",
} as const;

export type SoundName = keyof typeof FILES;

const VOLUME = 0.15;

class SoundEngine {
    private ctx: AudioContext | null = null;
    private master: GainNode | null = null;
    private buffers = new Map<SoundName, AudioBuffer>();
    private loading = new Map<SoundName, Promise<void>>();

    preload() {
        this.context();
        (Object.keys(FILES) as SoundName[]).forEach((name) =>
            this.load(name),
        );
    }

    play(name: SoundName) {
        if (!this.context()) return;

        const buffered = this.buffers.get(name);
        if (buffered) {
            this.fire(buffered);
            return;
        }

        void this.load(name).then(() => {
            const loaded = this.buffers.get(name);
            if (loaded) this.fire(loaded);
        });
    }

    private fire(buffer: AudioBuffer) {
        const ctx = this.ctx;
        const master = this.master;
        if (!ctx || !master) return;

        if (ctx.state === "suspended") void ctx.resume();

        const source = ctx.createBufferSource();
        source.buffer = buffer;
        source.connect(master);
        source.start();
    }

    private context(): AudioContext | null {
        if (this.ctx) {
            if (this.ctx.state === "suspended") void this.ctx.resume();
            return this.ctx;
        }

        const Ctor =
            window.AudioContext ??
            (window as unknown as { webkitAudioContext?: typeof AudioContext })
                .webkitAudioContext;
        if (!Ctor) return null;

        const ctx = new Ctor();
        const master = ctx.createGain();
        master.gain.value = VOLUME;
        master.connect(ctx.destination);

        this.ctx = ctx;
        this.master = master;
        return ctx;
    }

    private load(name: SoundName): Promise<void> {
        const pending = this.loading.get(name);
        if (pending) return pending;
        if (!this.ctx) return Promise.resolve();

        const ctx = this.ctx;
        const task = (async () => {
            try {
                const res = await fetch(FILES[name]);
                if (!res.ok) return;
                const buffer = await ctx.decodeAudioData(await res.arrayBuffer());
                this.buffers.set(name, buffer);
            } catch {
                return;
            }
        })();

        this.loading.set(name, task);
        return task;
    }
}

export const Sound = new SoundEngine();
