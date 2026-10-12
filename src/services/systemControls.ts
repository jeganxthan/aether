import { invoke } from "@tauri-apps/api/core";

export interface VolumeState {
  percent: number;
  muted: boolean;
}

export interface BrightnessState {
  percent: number;
}

export const systemControls = {
  getVolume: () =>
    invoke<VolumeState>("volume_get"),

  increaseVolume: () =>
    invoke<VolumeState>("volume_increase"),

  decreaseVolume: () =>
    invoke<VolumeState>("volume_decrease"),

  toggleMute: () =>
    invoke<VolumeState>("volume_toggle_mute"),

  getBrightness: () =>
    invoke<BrightnessState>("brightness_get"),

  increaseBrightness: () =>
    invoke<BrightnessState>("brightness_increase"),

  decreaseBrightness: () =>
    invoke<BrightnessState>("brightness_decrease"),
};