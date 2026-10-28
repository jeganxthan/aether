
import { invoke } from "@tauri-apps/api/core";

export interface AudioState {
  volumePercent: number;
  muted: boolean;
}

export const soundService = {
  getState: () => invoke<AudioState>("sound_get_state"),

  setVolume: (percent: number) =>
    invoke<AudioState>("sound_set_volume", {
      percent: Math.round(Math.max(0, Math.min(100, percent))),
    }),

  toggleMute: () => invoke<AudioState>("sound_toggle_mute"),

  increaseVolume: () => invoke<AudioState>("sound_increase_volume"),

  decreaseVolume: () => invoke<AudioState>("sound_decrease_volume"),
};
