import { useCallback, useEffect, useState } from "react";
import {
  Bluetooth as BluetoothIcon,
  BluetoothConnected,
  Check,
  ChevronRight,
  CircleAlert,
  LoaderCircle,
  Monitor,
  Mouse,
  RefreshCw,
  Smartphone,
  Speaker,
  Wifi,
} from "lucide-react";

import {
  bluetoothService,
  type BluetoothDevice,
} from "../services/bluetooth";

type DeviceAction = "pair" | "connect" | "disconnect";

function DeviceIcon({ name }: { name: string }) {
  const value = name.toLowerCase();

  if (
    value.includes("phone") ||
    value.includes("iphone") ||
    value.includes("android")
  ) {
    return <Smartphone size={19} />;
  }

  if (
    value.includes("speaker") ||
    value.includes("headphone") ||
    value.includes("airpod") ||
    value.includes("audio")
  ) {
    return <Speaker size={19} />;
  }

  if (value.includes("mouse")) {
    return <Mouse size={19} />;
  }

  if (
    value.includes("keyboard") ||
    value.includes("computer") ||
    value.includes("laptop")
  ) {
    return <Monitor size={19} />;
  }

  return <BluetoothIcon size={19} />;
}

export default function BluetoothPage() {
  const [enabled, setEnabled] = useState(false);
  const [devices, setDevices] = useState<BluetoothDevice[]>([]);
  const [scanning, setScanning] = useState(false);
  const [loading, setLoading] = useState(true);
  const [powerBusy, setPowerBusy] = useState(false);
  const [busyDevice, setBusyDevice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [status, foundDevices] = await Promise.all([
        bluetoothService.getStatus(),
        bluetoothService.getDevices(),
      ]);

      setEnabled(status.enabled);
      setDevices(foundDevices);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const toggleBluetooth = async () => {
    const next = !enabled;
    setPowerBusy(true);
    setError(null);

    try {
      const actualState = await bluetoothService.setEnabled(next);
      setEnabled(actualState);

      if (!actualState) {
        setScanning(false);
        setDevices([]);
      } else {
        await refresh();
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setPowerBusy(false);
    }
  };

  const scanDevices = async () => {
    setError(null);

    try {
      if (scanning) {
        await bluetoothService.stopScan();
        setScanning(false);
      } else {
        await bluetoothService.startScan();
        setScanning(true);
      }

      await refresh();
    } catch (err) {
      setError(String(err));
    }
  };

  const performAction = async (
    device: BluetoothDevice,
    action: DeviceAction,
  ) => {
    setBusyDevice(device.path);
    setError(null);

    try {
      if (action === "pair") {
        await bluetoothService.pair(device.path);
      } else if (action === "connect") {
        await bluetoothService.connect(device.path);
      } else {
        await bluetoothService.disconnect(device.path);
      }

      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusyDevice(null);
    }
  };

  const connected = devices.filter((device) => device.connected);
  const paired = devices.filter(
    (device) => device.paired && !device.connected,
  );
  const available = devices.filter(
    (device) => !device.paired && !device.connected,
  );

  const deviceSection = (
    title: string,
    subtitle: string,
    list: BluetoothDevice[],
    action: DeviceAction,
    emptyMessage: string,
  ) => (
    <section className="mt-7">
      <div className="mb-3 flex items-center justify-between">
        <div>
          <h2 className="text-sm font-medium text-neutral-100">
            {title}
            <span className="ml-2 text-xs text-neutral-500">
              {list.length}
            </span>
          </h2>
          <p className="mt-1 text-xs text-neutral-500">{subtitle}</p>
        </div>
      </div>

      <div className="overflow-hidden rounded-xl border border-white/[0.09] bg-white/[0.025]">
        {list.length === 0 ? (
          <div className="flex min-h-[82px] items-center justify-center px-5 text-center text-sm text-neutral-500">
            {loading ? "Loading devices..." : emptyMessage}
          </div>
        ) : (
          list.map((device, index) => (
            <div
              key={device.path}
              className={`flex items-center gap-3 px-4 py-3.5 ${
                index !== list.length - 1
                  ? "border-b border-white/[0.07]"
                  : ""
              }`}
            >
              <div className="flex size-10 shrink-0 items-center justify-center rounded-xl border border-white/[0.08] bg-white/[0.045] text-neutral-300">
                <DeviceIcon name={device.name || device.address} />
              </div>

              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium text-neutral-100">
                  {device.name || "Unknown device"}
                </p>
                <p className="mt-1 truncate text-xs text-neutral-500">
                  {device.connected
                    ? "Connected"
                    : device.paired
                      ? device.trusted
                        ? "Paired · Trusted"
                        : "Paired"
                      : device.address}
                </p>
              </div>

              {busyDevice === device.path ? (
                <LoaderCircle
                  size={17}
                  className="animate-spin text-[var(--accent)]"
                />
              ) : (
                <button
                  type="button"
                  disabled={busyDevice !== null}
                  onClick={() => void performAction(device, action)}
                  className={`flex shrink-0 items-center gap-1.5 rounded-lg border px-3 py-2 text-xs font-medium transition-colors disabled:opacity-50 ${
                    action === "disconnect"
                      ? "border-white/10 text-neutral-300 hover:bg-white/[0.07]"
                      : "border-[var(--accent-border)] bg-[var(--accent-soft)] text-[var(--accent)] hover:brightness-125"
                  }`}
                >
                  {action === "pair"
                    ? "Pair"
                    : action === "connect"
                      ? "Connect"
                      : "Disconnect"}
                  {action !== "disconnect" && (
                    <ChevronRight size={14} />
                  )}
                </button>
              )}
            </div>
          ))
        )}
      </div>
    </section>
  );

  return (
    <div className="mx-auto max-w-4xl">
      <header className="flex items-start justify-between gap-4">
        <div>
          <div className="flex items-center gap-2 text-xs text-neutral-500">
            <span>Connectivity</span>
            <ChevronRight size={13} />
            <span className="text-neutral-300">Bluetooth</span>
          </div>

          <h1 className="mt-3 text-[30px] font-semibold tracking-[-0.8px]">
            Bluetooth
          </h1>

          <p className="mt-2 text-sm text-neutral-400">
            Connect and manage your wireless devices.
          </p>
        </div>

        <button
          type="button"
          onClick={() => void refresh()}
          disabled={loading}
          aria-label="Refresh devices"
          className="mt-8 flex size-10 shrink-0 items-center justify-center rounded-xl border border-white/10 bg-white/[0.035] text-neutral-300 transition hover:bg-white/[0.08] disabled:opacity-50"
        >
          <RefreshCw
            size={16}
            className={loading ? "animate-spin" : ""}
          />
        </button>
      </header>

      {error && (
        <div
          role="alert"
          className="mt-6 flex items-start gap-3 rounded-xl border border-red-400/20 bg-red-400/[0.06] p-4 text-sm text-red-200"
        >
          <CircleAlert size={18} className="mt-0.5 shrink-0" />
          <div className="min-w-0">
            <p className="font-medium">Bluetooth operation failed</p>
            <p className="mt-1 break-words text-xs text-red-200/70">
              {error}
            </p>
          </div>
        </div>
      )}

      <section className="mt-7 flex items-center gap-4 rounded-2xl border border-white/[0.09] bg-white/[0.035] p-5">
        <div
          className={`flex size-12 shrink-0 items-center justify-center rounded-xl border ${
            enabled
              ? "border-[var(--accent-border)] bg-[var(--accent-soft)] text-[var(--accent)]"
              : "border-white/10 bg-white/[0.04] text-neutral-400"
          }`}
        >
          {enabled ? (
            <BluetoothConnected size={23} />
          ) : (
            <BluetoothIcon size={23} />
          )}
        </div>

        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h2 className="text-sm font-medium">Bluetooth</h2>
            <span
              className={`size-1.5 rounded-full ${
                enabled ? "bg-[var(--accent)]" : "bg-neutral-600"
              }`}
            />
          </div>

          <p className="mt-1 text-xs text-neutral-400">
            {enabled
              ? "Your device is discoverable while scanning."
              : "Bluetooth is turned off."}
          </p>
        </div>
<button
            type="button"
            role="switch"
            aria-checked={enabled}
            aria-label="Enable Wi-Fi"
            disabled={powerBusy}
            onClick={toggleBluetooth}
            className={`relative h-7 w-12 shrink-0 rounded-full border transition-colors duration-200 disabled:cursor-wait disabled:opacity-60 ${enabled
              ? "border-[var(--accent-border)] bg-[var(--accent)]"
              : "border-white/15 bg-white/10"
              }`}
          >
            <span
              className={`absolute left-[3px] top-[3px] size-5 rounded-full bg-white shadow transition-transform duration-200 ${enabled ? "translate-x-5" : "translate-x-0"
                }`}
            />
          </button>
        
      </section>

      {enabled ? (
        <>
          <div className="mt-7 flex items-center justify-between gap-4">
            <div>
              <h2 className="text-sm font-medium">Nearby devices</h2>
              <p className="mt-1 text-xs text-neutral-500">
                Scan for Bluetooth devices around you.
              </p>
            </div>

            <button
              type="button"
              onClick={() => void scanDevices()}
              disabled={loading}
              className="flex shrink-0 items-center gap-2 rounded-xl bg-[var(--accent)] px-4 py-2.5 text-sm font-medium text-neutral-950 transition hover:brightness-110 disabled:opacity-50"
            >
              {scanning ? (
                <LoaderCircle size={16} className="animate-spin" />
              ) : (
                <Wifi size={16} />
              )}
              {scanning ? "Stop scan" : "Scan"}
            </button>
          </div>

          {scanning && (
            <p className="mt-3 flex items-center gap-2 text-xs text-neutral-400">
              <span className="size-1.5 animate-pulse rounded-full bg-[var(--accent)]" />
              Scanning for nearby devices…
            </p>
          )}

          {deviceSection(
            "Connected devices",
            "Devices currently connected to Aether",
            connected,
            "disconnect",
            "No connected devices.",
          )}

          {deviceSection(
            "Paired devices",
            "Previously paired devices",
            paired,
            "connect",
            "No paired devices yet.",
          )}

          {deviceSection(
            "Available devices",
            "New devices discovered nearby",
            available,
            "pair",
            scanning
              ? "Waiting for nearby devices…"
              : "No new devices found. Start a scan to discover devices.",
          )}

          <p className="mt-5 flex items-center gap-2 text-xs text-neutral-500">
            <Check size={14} />
            Device actions are handled by the system Bluetooth service.
          </p>
        </>
      ) : (
        <div className="mt-7 rounded-xl border border-white/[0.08] bg-white/[0.02] px-5 py-8 text-center">
          <BluetoothIcon
            size={25}
            className="mx-auto text-neutral-500"
          />
          <h2 className="mt-3 text-sm font-medium">
            Bluetooth is disabled
          </h2>
          <p className="mx-auto mt-2 max-w-sm text-xs leading-5 text-neutral-500">
            Turn on Bluetooth to discover nearby devices and manage
            your existing connections.
          </p>
        </div>
      )}
    </div>
  );
}