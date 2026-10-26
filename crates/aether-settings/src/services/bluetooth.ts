import { invoke } from "@tauri-apps/api/core";

export interface BluetoothDevice {
  path: string;
  address: string;
  name: string;
  paired: boolean;
  connected: boolean;
  trusted: boolean;
  rssi: number | null;
}

export interface BluetoothStatus {
  enabled: boolean;
}

export const bluetoothService = {
  getStatus: () =>
    invoke<BluetoothStatus>("bluetooth_status"),

  setEnabled: (enabled: boolean) =>
    invoke<boolean>("bluetooth_set_enabled", { enabled }),

  getDevices: () =>
    invoke<BluetoothDevice[]>("bluetooth_devices"),

  startScan: () =>
    invoke<void>("bluetooth_start_scan"),

  stopScan: () =>
    invoke<void>("bluetooth_stop_scan"),

  pair: (path: string) =>
    invoke<void>("bluetooth_pair", { path }),

  connect: (path: string) =>
    invoke<void>("bluetooth_connect", { path }),

  disconnect: (path: string) =>
    invoke<void>("bluetooth_disconnect", { path }),
};