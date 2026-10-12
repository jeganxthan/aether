import { Sun, VolumeX } from "lucide-react";

export type FeedbackMode = "volume" | "brightness" | null;
export type BrightnessDirection = "up" | "down" | null;

interface SystemFeedbackProps {
  mode: FeedbackMode;
  volume: number;
  muted: boolean;
  brightness: number;
  brightnessDirection: BrightnessDirection;
}

export default function SystemFeedback({
  mode,
  volume,
  muted,
  brightness,
  brightnessDirection,
}: SystemFeedbackProps) {
  if (!mode) return null;

  const isVolume = mode === "volume";
  const percent = isVolume ? volume : brightness;

  return (
    <div
      key={mode}
      className="flex min-w-0 flex-1 items-center gap-2.5"
    >
      <div className="flex h-6 w-6 shrink-0 items-center justify-center">
        {isVolume ? (
          muted ? (
            <VolumeX size={18} strokeWidth={1.8} />
          ) : (
            <img
              src={
                volume <= 35
                  ? "/icons/volume/audio-low-symbolic.svg"
                  : "/icons/volume/audio-high-symbolic.svg"
              }
              alt="Volume"
              className="h-[19px] w-[19px] object-contain"
            />
          )
        ) : (
          <Sun
            size={19}
            strokeWidth={1.8}
            className={
              brightnessDirection
                ? "animate-[aether-sun-rotate_600ms_ease-in-out]"
                : ""
            }
          />
        )}
      </div>

      <div className="flex min-w-0 flex-1 flex-col justify-center gap-1">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[10px] font-medium tracking-[0.14em] text-white/65">
            {isVolume ? (muted ? "MUTED" : "VOLUME") : "BRIGHTNESS"}
          </span>

          <span className="text-[11px] font-medium tabular-nums text-white/90">
            {muted && isVolume ? "—" : `${percent}%`}
          </span>
        </div>

        <div className="h-[3px] w-full overflow-hidden rounded-sm bg-white/15">
          <div
            className="h-full rounded-sm bg-white transition-[width] duration-300 ease-out"
            style={{ width: `${muted && isVolume ? 0 : percent}%` }}
          />
        </div>
      </div>
    </div>
  );
}