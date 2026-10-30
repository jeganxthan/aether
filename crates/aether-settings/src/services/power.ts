import { invoke } from "@tauri-apps/api/core";

export interface BatteryInfo {
  available: boolean;
  percentage: number | null;
  status: string | null;
  energyNowWh: number | null;
  energyFullWh: number | null;
  energyDesignWh: number | null;
  powerW: number | null;
  voltageV: number | null;
  currentA: number | null;
  cycleCount: number | null;
  healthPercent: number | null;
  timeRemainingMinutes: number | null;
}

export interface CoreUsage {
  name: string;
  usagePercent: number;
  frequencyMhz: number | null;
}

export interface CpuInfo {
  usagePercent: number;
  logicalCores: number;
  loadAverage: [number, number, number];
  cores: CoreUsage[];
}

export interface MemoryInfo {
  totalBytes: number;
  availableBytes: number;
  usedBytes: number;
  cachedBytes: number;
  buffersBytes: number;
  swapTotalBytes: number;
  swapUsedBytes: number;
  usagePercent: number;
  swapUsagePercent: number;
}

export interface TemperatureSensor {
  name: string;
  temperatureC: number;
}

export interface SensorInfo {
  cpuTemperatureC: number | null;
  temperatures: TemperatureSensor[];
}

export interface PowerProfile {
  active: string | null;
  available: string[];
  supported: boolean;
  backend: string;
  detail: string | null;
}

export interface PowerSnapshot {
  battery: BatteryInfo;
  cpu: CpuInfo;
  memory: MemoryInfo;
  sensors: SensorInfo;
  profile: PowerProfile;
}

export const powerService = {
  snapshot: () => invoke<PowerSnapshot>("power_get_snapshot"),

  setProfile: (profile: string) =>
    invoke<PowerSnapshot>("power_set_profile", { profile }),
};

export function formatBytes(bytes: number): string {
  if (bytes < 1024 ** 3) {
    return `${(bytes / 1024 ** 2).toFixed(0)} MB`;
  }

  return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
}

export function formatDuration(minutes: number | null): string {
  if (minutes == null || !Number.isFinite(minutes)) return "Unavailable";

  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;

  if (hours === 0) return `${remainingMinutes} min`;

  return `${hours}h ${remainingMinutes}m`;
}