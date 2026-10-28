
import { useCallback, useEffect, useState } from "react";
import {
  Volume2,
  Volume1,
  VolumeX,
  Minus,
  Plus,
  LoaderCircle,
  RefreshCw,
  Check,
} from "lucide-react";

import { soundService, type AudioState } from "../services/sound";

export default function Sound() {
  const [audio, setAudio] = useState<AudioState | null>(null);
  const [loading, setLoading] = useState(true);
  const [changing, setChanging] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const state = await soundService.getState();
      setAudio(state);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();

    const interval = window.setInterval(() => {
      void refresh();
    }, 2000);

    return () => window.clearInterval(interval);
  }, [refresh]);

  async function perform(
    action: () => Promise<AudioState>,
    successMessage?: string,
  ) {
    if (changing) return;

    setChanging(true);
    setError(null);
    setMessage(null);

    try {
      const next = await action();
      setAudio(next);

      if (successMessage) {
        setMessage(successMessage);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setChanging(false);
    }
  }

  const volume = audio?.volumePercent ?? 0;
  const muted = audio?.muted ?? false;

  const VolumeIcon = muted || volume === 0
    ? VolumeX
    : volume < 50
      ? Volume1
      : Volume2;

  return (
    <main className="space-y-8 text-neutral-100">
      <header>
        <h1 className="text-[30px] font-semibold tracking-[-0.8px]">
          Sound
        </h1>
        <p className="mt-2 text-sm text-neutral-400">
          Manage your audio output and volume.
        </p>
      </header>

      {error && (
        <div
          role="alert"
          className="rounded-xl border border-red-400/20 bg-red-400/[0.06] p-3 text-sm text-red-300"
        >
          {error}
          <p className="mt-1 text-xs text-red-300/70">
            Make sure PipeWire and wpctl are available in your session.
          </p>
        </div>
      )}

      {message && (
        <p role="status" className="flex items-center gap-2 text-sm text-neutral-300">
          <Check size={15} />
          {message}
        </p>
      )}

      <section className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-5">
        <div className="flex items-center gap-3">
          <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-white/[0.06]">
            <VolumeIcon size={21} className="text-neutral-200" />
          </div>

          <div className="min-w-0 flex-1">
            <h2 className="text-sm font-medium">Output volume</h2>
            <p className="mt-1 text-xs text-neutral-500">
              Default audio output
            </p>
          </div>

          <span className="text-xl font-medium tabular-nums">
            {loading ? "—" : `${volume}%`}
          </span>

          <button
            type="button"
            onClick={() => void refresh()}
            disabled={loading || changing}
            title="Refresh audio state"
            className="flex h-9 w-9 items-center justify-center rounded-lg text-neutral-400 transition hover:bg-white/[0.06] hover:text-white disabled:opacity-50"
          >
            {loading
              ? <LoaderCircle size={16} className="animate-spin" />
              : <RefreshCw size={15} />}
          </button>
        </div>

        <div className="mt-7 flex items-center gap-3">
          <button
            type="button"
            onClick={() =>
              void perform(() => soundService.decreaseVolume())
            }
            disabled={!audio || changing || volume === 0}
            aria-label="Decrease volume"
            className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-white/10 bg-white/[0.03] transition hover:bg-white/[0.08] disabled:opacity-40"
          >
            <Minus size={16} />
          </button>

          <input
            aria-label="Output volume"
            type="range"
            min={0}
            max={100}
            step={1}
            value={volume}
            disabled={!audio || changing}
            onChange={(event) => {
              const percent = Number(event.target.value);
              setAudio((current) =>
                current ? { ...current, volumePercent: percent } : current,
              );
            }}
            onPointerUp={(event) => {
              const percent = Number(event.currentTarget.value);
              void perform(() => soundService.setVolume(percent));
            }}
            onKeyUp={(event) => {
              if (event.key.startsWith("Arrow") || event.key === "Home" || event.key === "End") {
                const percent = Number(event.currentTarget.value);
                void perform(() => soundService.setVolume(percent));
              }
            }}
            className="h-1.5 min-w-0 flex-1 cursor-pointer accent-white disabled:cursor-not-allowed"
          />

          <button
            type="button"
            onClick={() =>
              void perform(() => soundService.increaseVolume())
            }
            disabled={!audio || changing || volume >= 100}
            aria-label="Increase volume"
            className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-white/10 bg-white/[0.03] transition hover:bg-white/[0.08] disabled:opacity-40"
          >
            <Plus size={16} />
          </button>
        </div>

        <div className="mt-2 flex justify-between text-xs text-neutral-500">
          <span>0%</span>
          <span>50%</span>
          <span>100%</span>
        </div>

        <div className="mt-5 border-t border-white/[0.08] pt-4">
          <button
            type="button"
            onClick={() =>
              void perform(
                () => soundService.toggleMute(),
                muted ? "Audio unmuted." : "Audio muted.",
              )
            }
            disabled={!audio || changing}
            className="flex w-full items-center justify-between gap-3 rounded-xl p-3 text-left transition hover:bg-white/[0.04] disabled:opacity-50"
          >
            <div className="flex items-center gap-3">
              <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-white/[0.05]">
                {muted
                  ? <VolumeX size={17} />
                  : <Volume2 size={17} />}
              </div>
              <div>
                <p className="text-sm font-medium">
                  {muted ? "Muted" : "Mute audio"}
                </p>
                <p className="mt-1 text-xs text-neutral-500">
                  Toggle the default output mute state.
                </p>
              </div>
            </div>

            <span
              className={[
                "relative h-6 w-11 rounded-full transition",
                muted ? "bg-white/20" : "bg-white/80",
              ].join(" ")}
            >
              <span
                className={[
                  "absolute top-1 h-4 w-4 rounded-full transition",
                  muted ? "left-1 bg-neutral-300" : "left-6 bg-neutral-900",
                ].join(" ")}
              />
            </span>
          </button>
        </div>

        {changing && (
          <div className="mt-3 flex items-center gap-2 text-xs text-neutral-500">
            <LoaderCircle size={14} className="animate-spin" />
            Updating audio…
          </div>
        )}
      </section>
    </main>
  );
}
