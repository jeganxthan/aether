
import { invoke } from "@tauri-apps/api/core";


export interface AudioDevice {
  name: string;
  description: string;
  isDefault: boolean;
  muted: boolean;
}

export interface AudioDevices {
  outputs: AudioDevice[];
  inputs: AudioDevice[];
  defaultOutput: string | null;
  defaultInput: string | null;
}

export interface MicrophoneState {
  muted: boolean;
}

export const soundService = {
  listDevices: () =>
    invoke<AudioDevices>("sound_list_devices"),

  selectOutput: (name: string) =>
    invoke<AudioDevices>("sound_select_output", { name }),

  selectInput: (name: string) =>
    invoke<AudioDevices>("sound_select_input", { name }),

  setMicrophoneMute: (muted: boolean) =>
    invoke<MicrophoneState>("sound_set_microphone_mute", { muted }),
};