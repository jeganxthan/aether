
import { Monitor, CheckCircle2, Power } from "lucide-react";
import type { DisplayOutput } from "../../services/display";

interface MonitorCardProps {
  display: DisplayOutput;
  selected: boolean;
  onClick: () => void;
}

export default function MonitorCard({
  display,
  selected,
  onClick,
}: MonitorCardProps) {
  const mode = display.currentMode;

  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={selected}
      className={[
        "w-full rounded-2xl border p-4 text-left transition",
        selected
          ? "border-[var(--accent)] bg-white/[0.055]"
          : "border-white/[0.08] bg-white/[0.02] hover:bg-white/[0.045]",
      ].join(" ")}
    >
      <div className="flex items-start gap-3">
        <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-white/[0.06]">
          <Monitor size={20} className="text-neutral-300" />
        </div>

        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-2">
            <p className="truncate text-sm font-medium">{display.name}</p>
            {selected && (
              <CheckCircle2 size={16} className="shrink-0 text-[var(--accent)]" />
            )}
          </div>

          <p className="mt-1 truncate text-xs text-neutral-500">
            {display.description || "Display output"}
          </p>
        </div>
      </div>

      <div className="mt-4 grid grid-cols-2 gap-3">
        <div className="rounded-xl bg-black/15 p-3">
          <p className="text-xs text-neutral-500">Resolution</p>
          <p className="mt-1 text-sm text-neutral-200">
            {mode ? `${mode.width} × ${mode.height}` : "Unavailable"}
          </p>
        </div>

        <div className="rounded-xl bg-black/15 p-3">
          <p className="text-xs text-neutral-500">Refresh rate</p>
          <p className="mt-1 text-sm text-neutral-200">
            {mode ? `${mode.refreshRate.toFixed(2)} Hz` : "Unavailable"}
          </p>
        </div>
      </div>

      <div className="mt-3 flex items-center gap-2 text-xs text-neutral-500">
        <Power size={13} />
        {display.enabled ? "Connected · Enabled" : "Connected · Disabled"}
      </div>
    </button>
  );
}
