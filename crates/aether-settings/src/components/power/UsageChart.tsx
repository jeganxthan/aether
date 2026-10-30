import { useId } from "react";
import { Activity } from "lucide-react";

interface UsageChartProps {
  title: string;
  subtitle: string;
  values: number[];
  max: number;
}

const WIDTH = 320;
const HEIGHT = 96;

function clamp(value: number, max: number) {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(max, value));
}

export default function UsageChart({
  title,
  subtitle,
  values,
  max,
}: UsageChartProps) {
  const gradientId = `usage-${useId().replace(/[^a-zA-Z0-9_-]/g, "")}`;
  const safeMax = max > 0 ? max : 100;

  const samples = values.map((value) => clamp(value, safeMax));
  const current = samples.length > 0 ? samples[samples.length - 1] : null;
  const average =
    samples.length > 0
      ? samples.reduce((total, value) => total + value, 0) / samples.length
      : null;

  const points = samples
    .map((value, index) => {
      const x =
        samples.length === 1
          ? WIDTH
          : (index / (samples.length - 1)) * WIDTH;
      const y = HEIGHT - (value / safeMax) * HEIGHT;

      return `${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");

  const area = points
    ? `0,${HEIGHT} ${points} ${WIDTH},${HEIGHT}`
    : "";

  return (
    <section className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2">
          <Activity size={16} className="shrink-0 text-neutral-400" />
          <div className="min-w-0">
            <h3 className="truncate text-sm font-medium">{title}</h3>
            <p className="truncate text-xs text-neutral-500">{subtitle}</p>
          </div>
        </div>

        <span className="shrink-0 text-lg font-semibold tabular-nums">
          {current == null ? "—" : `${current.toFixed(1)}%`}
        </span>
      </div>

      <div className="mt-4 h-24 w-full overflow-hidden rounded-xl bg-black/15">
        {points ? (
          <svg
            viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
            preserveAspectRatio="none"
            role="img"
            aria-label={`${title} history`}
            className="h-full w-full"
          >
            <defs>
              <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
                <stop
                  offset="0%"
                  stopColor="var(--accent)"
                  stopOpacity="0.35"
                />
                <stop
                  offset="100%"
                  stopColor="var(--accent)"
                  stopOpacity="0"
                />
              </linearGradient>
            </defs>

            <path d={area} fill={`url(#${gradientId})`} />
            <path
              d={`M ${points.split(" ").join(" L ")}`}
              fill="none"
              stroke="var(--accent)"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              vectorEffect="non-scaling-stroke"
            />
          </svg>
        ) : (
          <div className="flex h-full items-center justify-center text-xs text-neutral-500">
            Collecting samples…
          </div>
        )}
      </div>

      <div className="mt-3 flex items-center justify-between text-xs text-neutral-500">
        <span>Average {average == null ? "—" : `${average.toFixed(1)}%`}</span>
        <span>{samples.length} / 60 samples</span>
      </div>
    </section>
  );
}
