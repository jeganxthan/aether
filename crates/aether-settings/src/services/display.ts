
import { invoke } from "@tauri-apps/api/core";

export interface DisplayMode {
  width: number;
  height: number;
  refreshRate: number;
  current: boolean;
  preferred: boolean;
}

export interface DisplayOutput {
  name: string;
  description: string;
  enabled: boolean;
  modes: DisplayMode[];
  currentMode: DisplayMode | null;
  scale: number;
  transform: string;
  x: number;
  y: number;
}

export interface DisplaySettings {
  name: string;
  width: number;
  height: number;
  refreshRate: number;
  scale: number;
  transform: string;
  x: number;
  y: number;
}

export const displayService = {
  list: () => invoke<DisplayOutput[]>("display_list"),
  primary: () => invoke<string | null>("display_primary_get"),
  setPrimary: (name: string) =>
    invoke<void>("display_primary_set", { name }),

  apply: (settings: DisplaySettings) =>
    invoke<void>("display_apply", { ...settings }),
};
