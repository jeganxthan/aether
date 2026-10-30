import { useCallback, useEffect, useState } from "react";
import {
  Activity,
  Battery,
  BatteryCharging,
  BatteryWarning,
  Check,
  Cpu,
  Gauge,
  MemoryStick,
  RefreshCw,
  Thermometer,
  Zap,
} from "lucide-react";

import UsageChart from "../components/power/UsageChart";
import {
  formatBytes,
  formatDuration,
  powerService,
  type PowerSnapshot,
} from "../services/power";

const HISTORY_LENGTH = 60;
const POLL_INTERVAL = 2000;

function percent(value: number | null | undefined) {
  return value == null || !Number.isFinite(value)
    ? "—"
    : `${value.toFixed(1)}%`;
}

function metric(value: number | null | undefined, suffix = "") {
  return value == null || !Number.isFinite(value)
    ? "Unavailable"
    : `${value.toFixed(1)}${suffix}`;
}

function batteryStatus(status: string | null) {
  if (!status) return "Status unavailable";

  switch (status.toLowerCase()) {
    case "charging":
      return "Charging";
    case "discharging":
      return "Discharging";
    case "full":
      return "Fully charged";
    case "not charging":
      return "Connected · Not charging";
    default:
      return status;
  }
}

function StatCard({
  label,
  value,
  detail,
  icon: Icon,
}: {
  label: string;
  value: string;
  detail: string;
  icon: typeof Battery;
}) {
  return (
    <article className="min-w-0 rounded-2xl border border-white/[0.08] bg-white/[0.025] p-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm text-neutral-400">{label}</p>
        <Icon size={17} className="shrink-0 text-neutral-500" />
      </div>

      <p className="mt-3 truncate text-2xl font-semibold tracking-tight text-neutral-100">
        {value}
      </p>

      <p className="mt-1 truncate text-xs text-neutral-500">{detail}</p>
    </article>
  );
}

function ProgressBar({ value }: { value: number }) {
  const safe = Math.max(0, Math.min(100, value));

  return (
    <div
      className="h-1.5 overflow-hidden rounded-full bg-white/[0.07]"
      role="progressbar"
      aria-valuenow={safe}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <div
        className="h-full rounded-full bg-[var(--accent)] transition-[width] duration-500"
        style={{ width: `${safe}%` }}
      />
    </div>
  );
}

export default function Power() {
  const [snapshot, setSnapshot] = useState<PowerSnapshot | null>(null);
  const [cpuHistory, setCpuHistory] = useState<number[]>([]);
  const [memoryHistory, setMemoryHistory] = useState<number[]>([]);
  const [batteryHistory, setBatteryHistory] = useState<number[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [profileBusy, setProfileBusy] = useState(false);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const load = useCallback(async (showSpinner = false) => {
    if (showSpinner) setRefreshing(true);

    try {
      const result = await powerService.snapshot();

      setSnapshot(result);
      setError(null);
      setLastUpdated(new Date());

      setCpuHistory((history) =>
        [...history, result.cpu.usagePercent].slice(-HISTORY_LENGTH),
      );

      setMemoryHistory((history) =>
        [...history, result.memory.usagePercent].slice(-HISTORY_LENGTH),
      );

      if (result.battery.percentage != null) {
        setBatteryHistory((history) =>
          [...history, result.battery.percentage!].slice(-HISTORY_LENGTH),
        );
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    void load();

    const timer = window.setInterval(() => {
      void load();
    }, POLL_INTERVAL);

    return () => window.clearInterval(timer);
  }, [load]);

  async function changeProfile(profile: string) {
    setProfileBusy(true);
    setError(null);

    try {
      const result = await powerService.setProfile(profile);
      setSnapshot(result);
      setLastUpdated(new Date());
    } catch (err) {
      setError(String(err));
    } finally {
      setProfileBusy(false);
    }
  }

  if (loading && !snapshot) {
    return (
      <main className="space-y-4 text-neutral-100">
        <h1 className="text-[30px] font-semibold tracking-tight">Power</h1>
        <div className="flex items-center gap-2 py-10 text-sm text-neutral-400">
          <RefreshCw size={16} className="animate-spin" />
          Reading system metrics…
        </div>
      </main>
    );
  }

  const battery = snapshot?.battery;
  const cpu = snapshot?.cpu;
  const memory = snapshot?.memory;
  const sensors = snapshot?.sensors;
  const profile = snapshot?.profile;

  return (
    <main className="space-y-7 text-neutral-100">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 className="text-[30px] font-semibold tracking-[-0.8px]">
            Power
          </h1>
          <p className="mt-2 text-sm text-neutral-400">
            Battery health, system utilization, thermals, and power profiles.
          </p>
          <p className="mt-2 text-xs text-neutral-500">
            {lastUpdated
              ? `Updated ${lastUpdated.toLocaleTimeString()} · Refreshes every 2 seconds`
              : "Waiting for system metrics"}
          </p>
        </div>

        <button
          type="button"
          onClick={() => void load(true)}
          disabled={refreshing}
          className="flex h-10 items-center gap-2 rounded-xl border border-white/10 bg-white/[0.03] px-3 text-sm text-neutral-300 transition hover:bg-white/[0.07] disabled:opacity-50"
        >
          <RefreshCw
            size={15}
            className={refreshing ? "animate-spin" : ""}
          />
          Refresh
        </button>
      </header>

      {error && (
        <div
          role="alert"
          className="flex items-start gap-2 rounded-xl border border-red-400/20 bg-red-400/[0.06] p-3 text-sm text-red-300"
        >
          <BatteryWarning size={17} className="mt-0.5 shrink-0" />
          <span className="break-words">{error}</span>
        </div>
      )}

      {/* Overview */}
      <section className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <StatCard
          label="Battery"
          value={
            battery?.percentage == null
              ? "Unavailable"
              : `${Math.round(battery.percentage)}%`
          }
          detail={
            battery?.available
              ? batteryStatus(battery.status)
              : "No battery detected"
          }
          icon={BatteryCharging}
        />

        <StatCard
          label="CPU usage"
          value={percent(cpu?.usagePercent)}
          detail={`${cpu?.logicalCores ?? 0} logical CPUs`}
          icon={Cpu}
        />

        <StatCard
          label="Memory used"
          value={
            memory ? formatBytes(memory.usedBytes) : "Unavailable"
          }
          detail={
            memory
              ? `${percent(memory.usagePercent)} of ${formatBytes(memory.totalBytes)}`
              : "No memory data"
          }
          icon={MemoryStick}
        />

        <StatCard
          label="CPU temperature"
          value={
            sensors?.cpuTemperatureC == null
              ? "Unavailable"
              : `${sensors.cpuTemperatureC.toFixed(1)}°C`
          }
          detail="When exposed by the kernel"
          icon={Thermometer}
        />
      </section>

      {/* Battery */}
      <section className="space-y-4">
        <div>
          <h2 className="text-base font-medium">Battery & charging</h2>
          <p className="mt-1 text-sm text-neutral-500">
            Capacity, charging behavior, and battery wear.
          </p>
        </div>

        {!battery?.available ? (
          <div className="rounded-2xl border border-white/[0.08] bg-white/[0.02] p-5">
            <div className="flex items-center gap-3">
              <Battery size={22} className="text-neutral-500" />
              <div>
                <p className="text-sm font-medium">Battery not detected</p>
                <p className="mt-1 text-xs text-neutral-500">
                  This device may be a desktop, or its battery may not be
                  exposed under /sys/class/power_supply.
                </p>
              </div>
            </div>
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-4 xl:grid-cols-[1.1fr_1fr]">
            <section className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-5">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-sm text-neutral-400">Charge level</p>
                  <p className="mt-2 text-4xl font-semibold tracking-tight">
                    {battery.percentage == null
                      ? "—"
                      : `${Math.round(battery.percentage)}%`}
                  </p>
                  <p className="mt-2 text-sm text-neutral-400">
                    {batteryStatus(battery.status)}
                  </p>
                </div>

                {battery.status?.toLowerCase() === "charging" ? (
                  <BatteryCharging size={26} className="text-[var(--accent)]" />
                ) : (
                  <Battery size={26} className="text-neutral-400" />
                )}
              </div>

              <div className="mt-5">
                <ProgressBar value={battery.percentage ?? 0} />
              </div>

              <div className="mt-5 grid grid-cols-2 gap-4">
                <div>
                  <p className="text-xs text-neutral-500">Energy remaining</p>
                  <p className="mt-1 text-sm font-medium">
                    {metric(battery.energyNowWh, " Wh")}
                  </p>
                </div>

                <div>
                  <p className="text-xs text-neutral-500">Full-charge capacity</p>
                  <p className="mt-1 text-sm font-medium">
                    {metric(battery.energyFullWh, " Wh")}
                  </p>
                </div>

                <div>
                  <p className="text-xs text-neutral-500">Power rate</p>
                  <p className="mt-1 text-sm font-medium">
                    {metric(battery.powerW, " W")}
                  </p>
                </div>

                <div>
                  <p className="text-xs text-neutral-500">Estimated time</p>
                  <p className="mt-1 text-sm font-medium">
                    {formatDuration(battery.timeRemainingMinutes)}
                  </p>
                </div>
              </div>
            </section>

            <section className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-5">
              <h3 className="text-sm font-medium">Battery health</h3>
              <p className="mt-1 text-xs text-neutral-500">
                Reported capacity compared with design capacity.
              </p>

              <div className="mt-5">
                <div className="flex items-baseline justify-between gap-3">
                  <span className="text-3xl font-semibold">
                    {percent(battery.healthPercent)}
                  </span>
                  <span className="text-xs text-neutral-500">Estimated health</span>
                </div>

                <div className="mt-3">
                  <ProgressBar value={battery.healthPercent ?? 0} />
                </div>
              </div>

              <div className="mt-5 space-y-3">
                {[
                  ["Design capacity", metric(battery.energyDesignWh, " Wh")],
                  ["Cycle count", battery.cycleCount?.toString() ?? "Unavailable"],
                  ["Voltage", metric(battery.voltageV, " V")],
                  ["Current", metric(battery.currentA, " A")],
                ].map(([label, value]) => (
                  <div
                    key={label}
                    className="flex items-center justify-between gap-3 border-b border-white/[0.06] pb-3 last:border-0 last:pb-0"
                  >
                    <span className="text-xs text-neutral-500">{label}</span>
                    <span className="text-sm tabular-nums text-neutral-200">
                      {value}
                    </span>
                  </div>
                ))}
              </div>
            </section>
          </div>
        )}
      </section>

      {/* Analytics */}
      <section className="space-y-4">
        <div>
          <h2 className="text-base font-medium">Live analytics</h2>
          <p className="mt-1 text-sm text-neutral-500">
            Rolling history collected while this page is open.
          </p>
        </div>

        <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
          <UsageChart
            title="CPU utilization"
            subtitle="Total logical CPU usage"
            values={cpuHistory}
            max={100}
          />

          <UsageChart
            title="Memory utilization"
            subtitle="Used memory as a percentage of total RAM"
            values={memoryHistory}
            max={100}
          />

          {batteryHistory.length > 1 && (
            <UsageChart
              title="Battery charge"
              subtitle="Observed charge level"
              values={batteryHistory}
              max={100}
            />
          )}

          <section className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-4">
            <div className="flex items-center gap-2">
              <Activity size={16} className="text-neutral-400" />
              <h3 className="text-sm font-medium">System load average</h3>
            </div>

            <div className="mt-5 grid grid-cols-3 gap-3">
              {(["1 min", "5 min", "15 min"] as const).map((label, index) => (
                <div key={label} className="rounded-xl bg-black/15 p-3">
                  <p className="text-xs text-neutral-500">{label}</p>
                  <p className="mt-2 text-xl font-semibold tabular-nums">
                    {cpu?.loadAverage[index]?.toFixed(2) ?? "—"}
                  </p>
                </div>
              ))}
            </div>

            <p className="mt-3 text-xs leading-relaxed text-neutral-500">
              Load average represents runnable and uninterruptible tasks; it
              is not a CPU percentage. Compare it with your logical CPU count.
            </p>
          </section>
        </div>
      </section>

      {/* Per-core */}
      <section className="space-y-4">
        <div>
          <h2 className="text-base font-medium">Per-core activity</h2>
          <p className="mt-1 text-sm text-neutral-500">
            Current utilization and frequency where available.
          </p>
        </div>

        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {(cpu?.cores ?? []).map((core) => (
            <div
              key={core.name}
              className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-4"
            >
              <div className="flex items-center justify-between gap-3">
                <span className="text-sm font-medium">{core.name}</span>
                <span className="text-sm tabular-nums text-neutral-300">
                  {percent(core.usagePercent)}
                </span>
              </div>

              <div className="mt-3">
                <ProgressBar value={core.usagePercent} />
              </div>

              <p className="mt-3 text-xs text-neutral-500">
                {core.frequencyMhz == null
                  ? "Frequency unavailable"
                  : `${core.frequencyMhz.toFixed(0)} MHz`}
              </p>
            </div>
          ))}
        </div>
      </section>

      {/* Memory */}
      <section className="space-y-4">
        <div>
          <h2 className="text-base font-medium">Memory breakdown</h2>
          <p className="mt-1 text-sm text-neutral-500">
            Physical memory, cache, buffers, and swap.
          </p>
        </div>

        <section className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-5">
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-2xl font-semibold">
                {memory ? formatBytes(memory.usedBytes) : "—"}
              </p>
              <p className="mt-1 text-xs text-neutral-500">
                Used of {memory ? formatBytes(memory.totalBytes) : "—"}
              </p>
            </div>

            <p className="text-sm text-neutral-300">
              {percent(memory?.usagePercent)}
            </p>
          </div>

          <div className="mt-4">
            <ProgressBar value={memory?.usagePercent ?? 0} />
          </div>

          <div className="mt-5 grid grid-cols-2 gap-4 lg:grid-cols-4">
            {[
              ["Available", memory ? formatBytes(memory.availableBytes) : "—"],
              ["Cached", memory ? formatBytes(memory.cachedBytes) : "—"],
              ["Buffers", memory ? formatBytes(memory.buffersBytes) : "—"],
              [
                "Swap used",
                memory
                  ? `${formatBytes(memory.swapUsedBytes)} / ${formatBytes(memory.swapTotalBytes)}`
                  : "—",
              ],
            ].map(([label, value]) => (
              <div key={label} className="rounded-xl bg-black/15 p-3">
                <p className="text-xs text-neutral-500">{label}</p>
                <p className="mt-2 text-sm font-medium">{value}</p>
              </div>
            ))}
          </div>

          <div className="mt-5">
            <div className="flex items-center justify-between gap-3">
              <span className="text-xs text-neutral-400">Swap utilization</span>
              <span className="text-xs text-neutral-300">
                {percent(memory?.swapUsagePercent)}
              </span>
            </div>

            <div className="mt-2">
              <ProgressBar value={memory?.swapUsagePercent ?? 0} />
            </div>
          </div>
        </section>
      </section>

      {/* Power profile */}
      <section className="space-y-4">
        <div>
          <h2 className="text-base font-medium">Power profile</h2>
          <p className="mt-1 text-sm text-neutral-500">
            Choose a system power policy supported by your Linux installation.
          </p>
        </div>

        <section className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-5">
          <div className="flex items-start gap-3">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-white/[0.05]">
              <Zap size={18} />
            </div>

            <div className="min-w-0 flex-1">
              <h3 className="text-sm font-medium">System performance mode</h3>
              <p className="mt-1 text-xs leading-relaxed text-neutral-500">
                Available modes depend on your hardware and power-profiles-daemon.
              </p>

              {!profile?.supported ? (
                <p className="mt-4 text-sm text-neutral-400">
                  No switchable power profiles were detected. Aether checked
                  power-profiles-daemon, CPU frequency policies, and firmware
                  controls on this system.
                </p>
              ) : (
                <div className="mt-4 grid grid-cols-1 gap-3 sm:grid-cols-3">
                  {[
                    {
                      id: "power-saver",
                      label: "Power saver",
                      detail: "Prioritize battery runtime",
                    },
                    {
                      id: "balanced",
                      label: "Balanced",
                      detail: "Everyday use",
                    },
                    {
                      id: "performance",
                      label: "Performance",
                      detail: "Prioritize responsiveness",
                    },
                  ]
                    .filter((item) => profile.available.includes(item.id))
                    .map((item) => {
                      const active = profile.active === item.id;

                      return (
                        <button
                          key={item.id}
                          type="button"
                          disabled={profileBusy || active}
                          onClick={() => void changeProfile(item.id)}
                          className={[
                            "rounded-xl border p-3 text-left transition",
                            active
                              ? "border-[var(--accent)] bg-white/[0.06]"
                              : "border-white/[0.08] hover:bg-white/[0.04]",
                            "disabled:cursor-not-allowed disabled:opacity-60",
                          ].join(" ")}
                        >
                          <div className="flex items-center justify-between gap-2">
                            <span className="text-sm font-medium">
                              {item.label}
                            </span>
                            {active && (
                              <Check
                                size={15}
                                className="text-[var(--accent)]"
                              />
                            )}
                          </div>

                          <p className="mt-2 text-xs text-neutral-500">
                            {item.detail}
                          </p>
                        </button>
                      );
                    })}
                </div>
              )}
              {profile?.supported && (
                <p className="mt-4 text-xs text-neutral-500">
                  Backend: {profile.backend.replace(/-/g, " ")}
                  {profile.detail ? ` · ${profile.detail}` : ""}
                </p>
              )}
            </div>
          </div>
        </section>
      </section>

      {/* Sensors */}
      <section className="space-y-4">
        <div>
          <h2 className="text-base font-medium">Thermal sensors</h2>
          <p className="mt-1 text-sm text-neutral-500">
            Temperatures exposed through Linux thermal zones.
          </p>
        </div>

        {!sensors?.temperatures.length ? (
          <div className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-4 text-sm text-neutral-500">
            No thermal-zone readings available. Your hardware may expose
            temperatures through a different sensor interface.
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
            {sensors.temperatures.map((sensor, index) => (
              <div
                key={`${sensor.name}-${index}`}
                className="flex items-center justify-between gap-3 rounded-xl border border-white/[0.08] bg-white/[0.025] p-4"
              >
                <div className="flex min-w-0 items-center gap-3">
                  <Thermometer size={17} className="shrink-0 text-neutral-500" />
                  <span className="truncate text-sm text-neutral-300">
                    {sensor.name}
                  </span>
                </div>

                <span className="shrink-0 text-sm font-medium tabular-nums">
                  {sensor.temperatureC.toFixed(1)}°C
                </span>
              </div>
            ))}
          </div>
        )}
      </section>

      <div className="flex items-center gap-2 border-t border-white/[0.06] pt-4 text-xs text-neutral-500">
        <Gauge size={14} />
        Metrics are read from Linux kernel interfaces. Unsupported values are
        shown as unavailable, not estimated.
      </div>
    </main>
  );
}