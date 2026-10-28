import { useCallback, useEffect, useRef, useState } from "react";
import {
  AlertCircle,
  Check,
  ChevronDown,
  LoaderCircle,
  Monitor,
  RefreshCw,
  RotateCw,
  Settings2,
} from "lucide-react";

import MonitorCard from "../components/display/MonitorCard";
import MonitorLayout, {
  type DisplayPosition,
} from "../components/display/MonitorLayout";
import {
  displayService,
  type DisplayOutput,
  type DisplaySettings,
} from "../services/display";

const TRANSFORMS = [
  { value: "normal", label: "Landscape" },
  { value: "90", label: "Portrait (90°)" },
  { value: "180", label: "Landscape (180°)" },
  { value: "270", label: "Portrait (270°)" },
  { value: "flipped", label: "Flipped" },
  { value: "flipped-90", label: "Flipped portrait (90°)" },
  { value: "flipped-180", label: "Flipped landscape (180°)" },
  { value: "flipped-270", label: "Flipped portrait (270°)" },
];

function modeKey(mode: {
  width: number;
  height: number;
  refreshRate: number;
}) {
  return `${mode.width}x${mode.height}@${mode.refreshRate.toFixed(3)}`;
}

export default function Display() {
  const [displays, setDisplays] = useState<DisplayOutput[]>([]);
  const [selectedName, setSelectedName] = useState("");
  const [draft, setDraft] = useState<DisplaySettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [layoutPositions, setLayoutPositions] = useState<
    Record<string, DisplayPosition>
  >({});
  const [primaryName, setPrimaryName] = useState("");
  const [savedPrimaryName, setSavedPrimaryName] = useState("");
  const selectedNameRef = useRef("");
  const selected = displays.find((display) => display.name === selectedName);

  const loadDisplays = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const [items, savedPrimary] = await Promise.all([
        displayService.list(),
        displayService.primary(),
      ]);
      setDisplays(items);

      const active = items.filter((item) => item.enabled);
      const primary =
        active.find((item) => item.name === savedPrimary)?.name ??
        active.find((item) => item.x === 0 && item.y === 0)?.name ??
        active[0]?.name ??
        "";
      setPrimaryName(primary);
      setSavedPrimaryName(savedPrimary ?? primary);
      setLayoutPositions(
        Object.fromEntries(
          active.map((item) => [item.name, { x: item.x, y: item.y }]),
        ),
      );

      const next =
        items.find((item) => item.name === selectedNameRef.current) ??
        items.find((item) => item.enabled) ??
        items[0];
      if (next) {
        selectedNameRef.current = next.name;
        setSelectedName(next.name);
        const mode = next.currentMode ?? next.modes[0];

        setDraft(
          mode
            ? {
              name: next.name,
              width: mode.width,
              height: mode.height,
              refreshRate: mode.refreshRate,
              scale: next.scale,
              transform: next.transform,
              x: next.x,
              y: next.y,
            }
            : null,
        );
      } else {
        setSelectedName("");
        setDraft(null);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadDisplays();
  }, [loadDisplays]);

  function selectDisplay(display: DisplayOutput) {
    selectedNameRef.current = display.name;
    setSelectedName(display.name);
    const mode = display.currentMode ?? display.modes[0];

    setDraft(
      mode
        ? {
          name: display.name,
          width: mode.width,
          height: mode.height,
          refreshRate: mode.refreshRate,
          scale: display.scale,
          transform: display.transform,
          x: layoutPositions[display.name]?.x ?? display.x,
          y: layoutPositions[display.name]?.y ?? display.y,
        }
        : null,
    );

    setError(null);
    setMessage(null);
  }

  function updateDraft(patch: Partial<DisplaySettings>) {
    const name = draft?.name;
    setDraft((current) => (current ? { ...current, ...patch } : current));
    if (name && (patch.x !== undefined || patch.y !== undefined)) {
      setLayoutPositions((current) => ({
        ...current,
        [name]: {
          x: patch.x ?? current[name]?.x ?? draft?.x ?? 0,
          y: patch.y ?? current[name]?.y ?? draft?.y ?? 0,
        },
      }));
    }
    setMessage(null);
  }

  function updateLayout(positions: Record<string, DisplayPosition>) {
    setLayoutPositions(positions);
    setDraft((current) => {
      const position = current ? positions[current.name] : undefined;
      if (!current || !position) return current;
      return { ...current, x: position.x, y: position.y };
    });
  }

  async function applySettings() {
    if (!draft || !primaryName) return;

    setApplying(true);
    setError(null);
    setMessage(null);

    try {
      const enabled = displays
        .filter((display) => display.enabled)
        .sort((left, right) => {
          if (left.name === primaryName) return -1;
          if (right.name === primaryName) return 1;
          return 0;
        });

      for (const display of enabled) {
        const isSelected = display.name === draft.name;
        const mode = isSelected
          ? {
              width: draft.width,
              height: draft.height,
              refreshRate: draft.refreshRate,
            }
          : display.currentMode;
        if (!mode) continue;

        const position = layoutPositions[display.name] ?? {
          x: display.x,
          y: display.y,
        };
        const scale = isSelected ? draft.scale : display.scale;
        const transform = isSelected ? draft.transform : display.transform;

        await displayService.apply({
          name: display.name,
          width: mode.width,
          height: mode.height,
          refreshRate: mode.refreshRate,
          scale,
          transform,
          x: display.name === primaryName ? 0 : position.x,
          y: display.name === primaryName ? 0 : position.y,
        });
      }

      await displayService.setPrimary(primaryName);
      setSavedPrimaryName(primaryName);
      setMessage("Monitor arrangement applied.");
      await loadDisplays();
    } catch (err) {
      setError(String(err));
    } finally {
      setApplying(false);
    }
  }

  const hasChanges =
    !!draft &&
    !!selected &&
    (primaryName !== savedPrimaryName ||
      displays.some((display) => {
        if (!display.enabled) return false;
        const position = layoutPositions[display.name] ?? {
          x: display.x,
          y: display.y,
        };
        return position.x !== display.x || position.y !== display.y;
      }) ||
      draft.width !== selected.currentMode?.width ||
      draft.height !== selected.currentMode?.height ||
      Math.abs(draft.refreshRate - (selected.currentMode?.refreshRate ?? 0)) > 0.05 ||
      Math.abs(draft.scale - selected.scale) > 0.001 ||
      draft.transform !== selected.transform ||
      draft.x !== selected.x ||
      draft.y !== selected.y);

  return (
    <main className="space-y-8 text-neutral-100">
      <header>
        <h1 className="text-[30px] font-semibold tracking-[-0.8px]">
          Display
        </h1>
        <p className="mt-2 text-sm text-neutral-400">
          Configure your monitors, resolution, scaling, and orientation.
        </p>
      </header>

      {error && (
        <div
          role="alert"
          className="flex items-start gap-2 rounded-xl border border-red-400/20 bg-red-400/[0.06] p-3 text-sm text-red-300"
        >
          <AlertCircle size={17} className="mt-0.5 shrink-0" />
          <span className="break-words">{error}</span>
        </div>
      )}

      {message && (
        <div
          role="status"
          className="flex items-center gap-2 text-sm text-neutral-300"
        >
          <Check size={16} />
          {message}
        </div>
      )}

      <section className="space-y-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h2 className="text-base font-medium">Connected displays</h2>
            <p className="mt-1 text-sm text-neutral-500">
              Select a monitor to configure it.
            </p>
          </div>

          <button
            type="button"
            onClick={() => void loadDisplays()}
            disabled={loading || applying}
            title="Refresh displays"
            className="flex h-10 w-10 items-center justify-center rounded-xl border border-white/10 bg-white/[0.03] text-neutral-300 hover:bg-white/[0.07] disabled:opacity-50"
          >
            <RefreshCw
              size={16}
              className={loading ? "animate-spin" : ""}
            />
          </button>
        </div>
        <MonitorLayout
          displays={displays.filter((display) => display.enabled)}
          selectedName={selectedName}
          primaryName={primaryName}
          onSelect={(name) => {
            const display = displays.find((item) => item.name === name);
            if (display) selectDisplay(display);
          }}
          onPrimaryChange={setPrimaryName}
          positions={layoutPositions}
          onLayoutChange={updateLayout}
        />
        {loading ? (
          <div className="flex items-center justify-center gap-2 py-10 text-sm text-neutral-400">
            <LoaderCircle size={18} className="animate-spin" />
            Detecting displays…
          </div>
        ) : displays.length === 0 ? (
          <div className="rounded-2xl border border-dashed border-white/10 py-10 text-center">
            <Monitor size={26} className="mx-auto text-neutral-500" />
            <p className="mt-3 text-sm text-neutral-300">
              No displays detected
            </p>
            <p className="mt-1 text-xs text-neutral-500">
              Check that wlr-randr works in your Wayland session.
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
            {displays.map((display) => (
              <MonitorCard
                key={display.name}
                display={display}
                selected={selectedName === display.name}
                onClick={() => selectDisplay(display)}
              />
            ))}
          </div>
        )}
      </section>

      {selected && draft && (
        <section className="space-y-5 rounded-2xl border border-white/[0.08] bg-white/[0.02] p-5">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-white/[0.05]">
              <Settings2 size={18} />
            </div>
            <div>
              <h2 className="text-base font-medium">Display configuration</h2>
              <p className="mt-1 text-xs text-neutral-500">
                {selected.name}
              </p>
            </div>
          </div>

          <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
            <label className="space-y-2">
              <span className="block text-sm text-neutral-300">
                Resolution
              </span>
              <span className="group relative block">
                <select
                  value={`${draft.width}x${draft.height}`}
                  disabled={applying}
                  onChange={(event) => {
                    const [width, height] = event.target.value
                      .split("x")
                      .map(Number);

                    const available = selected.modes
                      .filter(
                        (mode) => mode.width === width && mode.height === height,
                      )
                      .sort((a, b) => b.refreshRate - a.refreshRate);

                    const best =
                      available.find((mode) => mode.preferred) ?? available[0];

                    if (best) {
                      updateDraft({
                        width: best.width,
                        height: best.height,
                        refreshRate: best.refreshRate,
                      });
                    }
                  }}
                  className="w-full appearance-none rounded-xl border border-white/10 bg-gradient-to-b from-white/[0.055] to-white/[0.025] px-3 py-2.5 pr-10 text-sm text-neutral-100 shadow-inner shadow-black/20 outline-none transition hover:border-white/20 focus:border-[var(--accent)]/70 focus:ring-2 focus:ring-[var(--accent)]/15 disabled:cursor-wait disabled:opacity-50 [color-scheme:dark]"
                >
                  {[...new Map(
                    selected.modes.map((mode) => [
                      `${mode.width}x${mode.height}`,
                      mode,
                    ]),
                  ).values()]
                    .sort((a, b) => b.width * b.height - a.width * a.height)
                    .map((mode) => (
                      <option
                        key={`${mode.width}x${mode.height}`}
                        value={`${mode.width}x${mode.height}`}
                      >
                        {mode.width} × {mode.height}
                        {mode.preferred ? " · Recommended" : ""}
                      </option>
                    ))}
                </select>
                <ChevronDown
                  size={15}
                  className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-neutral-500 transition group-hover:text-neutral-300"
                />
              </span>
            </label>

            <label className="space-y-2">
              <span className="block text-sm text-neutral-300">
                Refresh rate
              </span>
              <span className="group relative block">
                <select
                  value={modeKey(draft)}
                  disabled={applying}
                  onChange={(event) => {
                    const mode = selected.modes.find(
                      (item) => modeKey(item) === event.target.value,
                    );

                    if (mode) {
                      updateDraft({
                        width: mode.width,
                        height: mode.height,
                        refreshRate: mode.refreshRate,
                      });
                    }
                  }}
                  className="w-full appearance-none rounded-xl border border-white/10 bg-gradient-to-b from-white/[0.055] to-white/[0.025] px-3 py-2.5 pr-10 text-sm text-neutral-100 shadow-inner shadow-black/20 outline-none transition hover:border-white/20 focus:border-[var(--accent)]/70 focus:ring-2 focus:ring-[var(--accent)]/15 disabled:cursor-wait disabled:opacity-50 [color-scheme:dark]"
                >
                  {selected.modes
                    .filter(
                      (mode) =>
                        mode.width === draft.width &&
                        mode.height === draft.height,
                    )
                    .sort((a, b) => b.refreshRate - a.refreshRate)
                    .map((mode) => (
                      <option key={modeKey(mode)} value={modeKey(mode)}>
                        {mode.refreshRate.toFixed(2)} Hz
                        {mode.preferred ? " · Preferred" : ""}
                      </option>
                    ))}
                </select>
                <ChevronDown
                  size={15}
                  className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-neutral-500 transition group-hover:text-neutral-300"
                />
              </span>
            </label>

            <label className="space-y-2">
              <span className="block text-sm text-neutral-300">
                Scale
              </span>
              <span className="group relative block">
                <select
                  value={String(draft.scale)}
                  disabled={applying}
                  onChange={(event) =>
                    updateDraft({ scale: Number(event.target.value) })
                  }
                  className="w-full appearance-none rounded-xl border border-white/10 bg-gradient-to-b from-white/[0.055] to-white/[0.025] px-3 py-2.5 pr-10 text-sm text-neutral-100 shadow-inner shadow-black/20 outline-none transition hover:border-white/20 focus:border-[var(--accent)]/70 focus:ring-2 focus:ring-[var(--accent)]/15 disabled:cursor-wait disabled:opacity-50 [color-scheme:dark]"
                >
                  {[1, 1.25, 1.5, 1.75, 2].map((scale) => (
                    <option key={scale} value={scale}>
                      {Math.round(scale * 100)}%
                    </option>
                  ))}
                </select>
                <ChevronDown
                  size={15}
                  className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-neutral-500 transition group-hover:text-neutral-300"
                />
              </span>
            </label>

            <label className="space-y-2">
              <span className="block text-sm text-neutral-300">
                Orientation
              </span>
              <span className="group relative block">
                <RotateCw
                  size={15}
                  className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-neutral-400"
                />
                <select
                  value={draft.transform}
                  disabled={applying}
                  onChange={(event) =>
                    updateDraft({ transform: event.target.value })
                  }
                  className="w-full appearance-none rounded-xl border border-white/10 bg-gradient-to-b from-white/[0.055] to-white/[0.025] py-2.5 pl-9 pr-10 text-sm text-neutral-100 shadow-inner shadow-black/20 outline-none transition hover:border-white/20 focus:border-[var(--accent)]/70 focus:ring-2 focus:ring-[var(--accent)]/15 disabled:cursor-wait disabled:opacity-50 [color-scheme:dark]"
                >
                  {TRANSFORMS.map((transform) => (
                    <option key={transform.value} value={transform.value}>
                      {transform.label}
                    </option>
                  ))}
                </select>
                <ChevronDown
                  size={15}
                  className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-neutral-500 transition group-hover:text-neutral-300"
                />
              </span>
            </label>
          </div>

          <div className="space-y-3 border-t border-white/[0.08] pt-5">
            <div>
              <h3 className="text-sm font-medium">Monitor position</h3>
              <p className="mt-1 text-xs text-neutral-500">
                Set this display's position relative to the virtual desktop.
                The primary display stays at 0, 0.
              </p>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <label className="space-y-2">
                <span className="block text-xs text-neutral-400">
                  Horizontal (X)
                </span>
                <input
                  type="number"
                  value={draft.x}
                  disabled={applying || draft.name === primaryName}
                  onChange={(event) =>
                    updateDraft({ x: Number(event.target.value) })
                  }
                  className="w-full rounded-xl border border-white/10 bg-[#202124] px-3 py-2.5 text-sm outline-none focus:border-white/25"
                />
              </label>

              <label className="space-y-2">
                <span className="block text-xs text-neutral-400">
                  Vertical (Y)
                </span>
                <input
                  type="number"
                  value={draft.y}
                  disabled={applying || draft.name === primaryName}
                  onChange={(event) =>
                    updateDraft({ y: Number(event.target.value) })
                  }
                  className="w-full rounded-xl border border-white/10 bg-[#202124] px-3 py-2.5 text-sm outline-none focus:border-white/25"
                />
              </label>
            </div>
          </div>

          <div className="flex flex-wrap items-center justify-between gap-3 border-t border-white/[0.08] pt-5">
            <p className="text-xs text-neutral-500">
              Changes may briefly affect your desktop layout.
            </p>

            <button
              type="button"
              onClick={() => void applySettings()}
              disabled={!hasChanges || applying}
              className="
flex shrink-0 items-center gap-2 rounded-xl bg-[var(--accent)] px-4 py-2.5 text-sm font-medium text-neutral-950 transition hover:brightness-110 disabled:opacity-50"
            >
              {applying ? (
                <LoaderCircle size={16} className="animate-spin" />
              ) : (
                <Check size={16} />
              )}
              {applying ? "Applying…" : "Apply changes"}
            </button>
          </div>
        </section>
      )}
    </main>
  );
}
