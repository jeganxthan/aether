import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Wifi,
  WifiOff,
  LoaderCircle,
  AlertCircle,
  RefreshCw,
  LockKeyhole,
  Check,
  Signal,
} from "lucide-react";

import {
  loadWifiNetworks,
  getCachedNetworks,
  getCachedWifiEnabled,
  setCachedWifiEnabled,
} from "../services/wifiCache";

interface WifiNetwork {
  ssid: string;
  signal: number;
  security: string;
  connected: boolean;
}

export default function WifiPage() {
  const [enabled, setEnabled] = useState(
    () => getCachedWifiEnabled() ?? false,
  );

  const [loading, setLoading] = useState(
    () => getCachedWifiEnabled() === null,
  );

  const [networks, setNetworks] = useState<WifiNetwork[]>(
    () => getCachedNetworks() ?? [],
  );

  const [scanning, setScanning] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [scanError, setScanError] = useState<string | null>(null);
  const [expandedSsid, setExpandedSsid] = useState<string | null>(null);
  const [password, setPassword] = useState("");
  const [connectingSsid, setConnectingSsid] = useState<string | null>(null);
  const [forgettingSsid, setForgettingSsid] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  const scanNetworks = useCallback(async (forceRefresh = false) => {
    setScanning(true);
    setScanError(null);

    try {
      const result = await loadWifiNetworks(
        () => invoke<WifiNetwork[]>("wifi_scan_networks"),
        forceRefresh,
      );

      setNetworks(result);
    } catch (err) {
      setScanError(String(err));
    } finally {
      setScanning(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function initialize() {
      try {
        const cachedState = getCachedWifiEnabled();
        const state =
          cachedState ?? await invoke<boolean>("wifi_is_enabled");

        if (cancelled) return;

        setEnabled(state);
        setCachedWifiEnabled(state);

        if (!state) {
          setNetworks([]);
          return;
        }

        const cached = getCachedNetworks();

        if (cached !== null) {
          setNetworks(cached);
          return;
        }

        setScanning(true);

        const result = await loadWifiNetworks(
          () => invoke<WifiNetwork[]>("wifi_scan_networks"),
        );

        if (!cancelled) setNetworks(result);
      } catch (err) {
        if (!cancelled) setScanError(String(err));
      } finally {
        if (!cancelled) {
          setLoading(false);
          setScanning(false);
        }
      }
    }

    void initialize();

    return () => {
      cancelled = true;
    };
  }, []);

  async function handleToggle() {
    if (loading || updating) return;

    const desiredState = !enabled;
    setUpdating(true);
    setError(null);

    try {
      const actualState = await invoke<boolean>("wifi_set_enabled", {
        enabled: desiredState,
      });

      setEnabled(actualState);
      setCachedWifiEnabled(actualState);

      if (actualState) {
        await scanNetworks(true);
      } else {
        setNetworks([]);
        setScanError(null);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setUpdating(false);
    }
  }
  async function handleConnect(network: WifiNetwork) {
    const isOpen = network.security.trim().toLowerCase() === "open";

    if (!isOpen && !password.trim()) {
      setActionError("Enter the Wi-Fi password.");
      return;
    }

    setConnectingSsid(network.ssid);
    setActionError(null);

    try {
      await invoke("wifi_connect_network", {
        ssid: network.ssid,
        password: isOpen ? "" : password,
      });

      setPassword("");
      setExpandedSsid(null);

      await scanNetworks(true);
    } catch (err) {
      setActionError(String(err));
    } finally {
      setConnectingSsid(null);
    }
  }

  async function handleForget(network: WifiNetwork) {
    setForgettingSsid(network.ssid);
    setActionError(null);

    try {
      await invoke("wifi_forget_network", {
        ssid: network.ssid,
      });

      setExpandedSsid(null);
      setPassword("");

      await scanNetworks(true);
    } catch (err) {
      setActionError(String(err));
    } finally {
      setForgettingSsid(null);
    }
  }
  function signalLabel(signal: number) {
    if (signal >= 75) return "Excellent signal";
    if (signal >= 50) return "Good signal";
    if (signal >= 25) return "Fair signal";
    return "Weak signal";
  }

  return (
    <div className="mx-auto w-full max-w-3xl space-y-7">
      <header>
        <div className="mb-3 flex items-center gap-3">
          <div className="grid h-12 w-12 place-items-center rounded-2xl border border-white/10 bg-white/[0.06]">
            <Wifi size={24} strokeWidth={1.6} />
          </div>

          <div>
            <h1 className="text-2xl font-semibold tracking-tight">
              Wi-Fi
            </h1>
            <p className="mt-1 text-sm text-white/45">
              Manage wireless connections and nearby networks.
            </p>
          </div>
        </div>
      </header>

      {/* Wi-Fi radio */}
      <section className="rounded-2xl border border-white/[0.12] bg-white/[0.045] p-5 backdrop-blur-2xl">
        <div className="flex items-center justify-between gap-5">
          <div className="flex items-center gap-3">
            <div className="grid size-10 place-items-center rounded-xl bg-white/[0.06]">
              {enabled ? (
                <Wifi size={19} />
              ) : (
                <WifiOff size={19} className="text-white/40" />
              )}
            </div>

            <div>
              <h2 className="text-sm font-medium">Wi-Fi</h2>
              <p className="mt-1 text-xs text-white/45">
                {loading
                  ? "Checking Wi-Fi status..."
                  : enabled
                    ? "Wireless radio is enabled"
                    : "Wireless radio is disabled"}
              </p>
            </div>
          </div>

          <button
            type="button"
            role="switch"
            aria-checked={enabled}
            aria-label="Enable Wi-Fi"
            disabled={loading || updating}
            onClick={handleToggle}
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
        </div>

        {updating && (
          <div className="mt-4 flex items-center gap-2 text-xs text-white/50">
            <LoaderCircle size={14} className="animate-spin" />
            Updating Wi-Fi...
          </div>
        )}

        {error && (
          <div className="mt-4 flex items-start gap-2 rounded-xl border border-red-400/20 bg-red-400/[0.07] p-3 text-xs text-red-200">
            <AlertCircle size={16} className="mt-0.5 shrink-0" />
            <div>
              <p className="font-medium">Could not update Wi-Fi</p>
              <p className="mt-1 break-words text-red-200/70">
                {error}
              </p>
            </div>
          </div>
        )}
      </section>

      {/* Available networks */}
      <section className="overflow-hidden rounded-2xl border border-white/[0.12] bg-white/[0.035] backdrop-blur-2xl">
        <div className="flex items-center justify-between border-b border-white/[0.08] px-5 py-4">
          <div>
            <h2 className="text-sm font-medium">Available networks</h2>
            <p className="mt-1 text-xs text-white/40">
              {enabled
                ? `${networks.length} network${networks.length === 1 ? "" : "s"} found`
                : "Turn on Wi-Fi to find nearby networks"}
            </p>
          </div>

          <button
            type="button"
            onClick={() => void scanNetworks(true)}
            disabled={!enabled || scanning || updating}
            aria-label="Refresh available networks"
            title="Refresh networks"
            className="grid size-9 place-items-center rounded-xl border border-white/10 bg-white/[0.04] text-white/70 transition hover:bg-white/[0.09] hover:text-white disabled:cursor-not-allowed disabled:opacity-35"
          >
            {scanning ? (
              <LoaderCircle size={16} className="animate-spin" />
            ) : (
              <RefreshCw size={16} />
            )}
          </button>
        </div>

        {scanError && (
          <div className="m-4 flex items-start gap-2 rounded-xl border border-red-400/20 bg-red-400/[0.07] p-3 text-xs text-red-200">
            <AlertCircle size={15} className="mt-0.5 shrink-0" />
            <p className="break-words">{scanError}</p>
          </div>
        )}

        {!enabled ? (
          <div className="flex flex-col items-center px-5 py-10 text-center">
            <WifiOff size={25} className="text-white/30" />
            <p className="mt-3 text-sm text-white/65">Wi-Fi is turned off</p>
            <p className="mt-1 text-xs text-white/35">
              Enable Wi-Fi to scan for nearby networks.
            </p>
          </div>
        ) : scanning && networks.length === 0 ? (
          <div className="flex flex-col items-center px-5 py-12 text-center">
            {/* Animated Wi-Fi indicator */}
            <div className="relative grid size-16 place-items-center">
              <span className="absolute inset-0 animate-ping rounded-full bg-[var(--accent)] opacity-[0.08]" />

              <div className="relative grid size-14 place-items-center rounded-2xl border border-[var(--accent-border)] bg-[var(--accent-soft)]">
                <Wifi
                  size={27}
                  strokeWidth={1.5}
                  className="text-[var(--accent)]"
                />
              </div>
            </div>

            <p className="mt-5 text-sm font-medium text-white/85">
              Scanning for networks
            </p>

            <p className="mt-1.5 text-xs text-white/40">
              Discovering nearby Wi-Fi connections
            </p>

            {/* Minimal animated progress line */}
            <div className="mt-5 h-1 w-36 overflow-hidden rounded-full bg-white/[0.07]">
              <div className="h-full w-1/2 animate-[wifi-scan_1.4s_ease-in-out_infinite] rounded-full bg-[var(--accent)]" />
            </div>
          </div>
        ) : (
          <div className="divide-y divide-white/[0.06]">
            {networks.map((network, index) => {
              const isExpanded = expandedSsid === network.ssid;
              const isConnected = network.connected;
              const isOpen =
                network.security.trim().toLowerCase() === "open";
              const isConnecting = connectingSsid === network.ssid;
              const isForgetting = forgettingSsid === network.ssid;

              return (
                <div key={`${network.ssid}-${index}`}>
                  {/* Clickable network row */}
                  <button
                    type="button"
                    onClick={() => {
                      setExpandedSsid(isExpanded ? null : network.ssid);
                      setPassword("");
                      setActionError(null);
                    }}
                    aria-expanded={isExpanded}
                    className="flex w-full items-center gap-3 px-5 py-4 text-left transition-colors hover:bg-white/[0.045]"
                  >
                    <div
                      className={`grid size-10 shrink-0 place-items-center rounded-xl ${isConnected
                          ? "border border-[var(--accent-border)] bg-[var(--accent-soft)] text-[var(--accent)]"
                          : "bg-white/[0.05] text-white/70"
                        }`}
                    >
                      <Wifi size={19} />
                    </div>

                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <p className="truncate text-sm font-medium">
                          {network.ssid || "Hidden network"}
                        </p>

                        {isConnected && (
                          <span className="inline-flex items-center gap-1 rounded-md bg-[var(--accent-soft)] px-1.5 py-0.5 text-[10px] font-medium text-[var(--accent)]">
                            <Check size={11} />
                            Connected
                          </span>
                        )}
                      </div>

                      <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-white/40">
                        <span>{network.security || "Security unknown"}</span>
                        <span>·</span>
                        <span>{signalLabel(network.signal)}</span>
                      </div>
                    </div>

                    <div
                      className="flex shrink-0 items-center gap-1.5 text-xs text-white/50"
                      title={`${network.signal}% signal strength`}
                    >
                      <Signal size={15} />
                      {network.signal}%
                      {!isOpen && <LockKeyhole size={13} className="ml-1" />}
                    </div>
                  </button>

                  {/* Expandable actions */}
                  {isExpanded && (
                    <div className="border-t border-white/[0.06] bg-black/[0.12] px-5 py-4">
                      {isConnected ? (
                        <div className="flex items-center justify-between gap-4">
                          <div>
                            <p className="text-sm font-medium text-white/85">
                              Connected to this network
                            </p>
                            <p className="mt-1 text-xs text-white/40">
                              Forget this saved connection.
                            </p>
                          </div>

                          <button
                            type="button"
                            disabled={isForgetting || updating}
                            onClick={() => void handleForget(network)}
                            className="shrink-0 rounded-xl border border-red-400/20 bg-red-400/[0.07] px-4 py-2 text-sm font-medium text-red-200 transition hover:bg-red-400/[0.14] disabled:opacity-50"
                          >
                            {isForgetting ? (
                              <span className="flex items-center gap-2">
                                <LoaderCircle size={14} className="animate-spin" />
                                Forgetting...
                              </span>
                            ) : (
                              "Forget"
                            )}
                          </button>
                        </div>
                      ) : (
                        <form
                          onSubmit={(event) => {
                            event.preventDefault();
                            void handleConnect(network);
                          }}
                          className="space-y-3"
                        >
                          <label className="block text-xs font-medium text-white/60">
                            {isOpen ? "Open network" : "Wi-Fi password"}
                          </label>

                          {!isOpen && (
                            <input
                              type="password"
                              value={password}
                              onChange={(event) => setPassword(event.target.value)}
                              placeholder="Enter network password"
                              autoComplete="current-password"
                              disabled={isConnecting}
                              className="w-full rounded-xl border border-white/10 bg-white/[0.04] px-3.5 py-3 text-sm text-white outline-none transition placeholder:text-white/25 focus:border-[var(--accent-border)] focus:bg-white/[0.06]"
                            />
                          )}

                          {isOpen && (
                            <p className="text-xs text-white/40">
                              This network does not require a password.
                            </p>
                          )}

                          <div className="flex justify-end">
                            <button
                              type="submit"
                              disabled={isConnecting || updating}
                              className="inline-flex items-center gap-2 rounded-xl bg-[var(--accent)] px-4 py-2.5 text-sm font-medium text-neutral-950 transition hover:brightness-110 disabled:cursor-wait disabled:opacity-50"
                            >
                              {isConnecting && (
                                <LoaderCircle size={15} className="animate-spin" />
                              )}
                              {isConnecting ? "Connecting..." : "Connect"}
                            </button>
                          </div>
                        </form>
                      )}

                      {actionError && (
                        <div className="mt-3 flex items-start gap-2 rounded-xl border border-red-400/20 bg-red-400/[0.07] p-3 text-xs text-red-200">
                          <AlertCircle size={15} className="mt-0.5 shrink-0" />
                          <p className="break-words">{actionError}</p>
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </section>

    </div>
  );
}