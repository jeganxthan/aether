interface WifiCardProps {
  enabled: boolean;
  connected: boolean;
  ssid?: string | null;
  signal?: number | null;
  security?: string;
  onClick?: () => void;
}

function getWifiIcon(
  signal: number | null | undefined,
  security?: string,
) {
  const isSecure =
    security &&
    security !== "--" &&
    security.trim() !== "";

  const prefix = isSecure
    ? "network-wireless-secure-signal"
    : "network-wireless-signal";

  if (signal == null || signal <= 0) {
    return `/icons/wifi/${prefix}-none-symbolic.svg`;
  }

  if (signal < 40) {
    return `/icons/wifi/${prefix}-weak-symbolic.svg`;
  }

  if (signal < 60) {
    return `/icons/wifi/${prefix}-ok-symbolic.svg`;
  }

  if (signal < 80) {
    return `/icons/wifi/${prefix}-good-symbolic.svg`;
  }

  return `/icons/wifi/${prefix}-excellent-symbolic.svg`;
}

const TILE =
  "rounded-xl border border-neutral-900 bg-neutral-950 px-3 py-3 text-left transition-colors hover:border-neutral-800";

export default function WifiCard({
  enabled,
  connected,
  ssid,
  signal,
  security,
  onClick,
}: WifiCardProps) {
  if (!enabled) {
    return (
      <button
        type="button"
        onClick={onClick}
        className={`w-[76px] shrink-0 ${TILE}`}
      >
        <img
          src="/icons/wifi/network-wireless-disconnected-symbolic.svg"
          alt=""
          className="h-4 w-4 opacity-70"
        />

        <p className="mt-2 text-[10px] tracking-[0.18em] text-neutral-600">
          OFF
        </p>
      </button>
    );
  }

  const wifiIcon = getWifiIcon(signal, security);

  return (
    <button
      type="button"
      onClick={onClick}
      className={`min-w-0 flex-1 ${TILE}`}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <img
            src={wifiIcon}
            alt=""
            className="h-4 w-4 shrink-0 opacity-70"
          />

          <p className="truncate text-xs text-neutral-300">
            {ssid || "Wi-Fi"}
          </p>
        </div>

        {connected && signal != null && (
          <span className="shrink-0 text-[11px] text-neutral-500">
            {signal}%
          </span>
        )}
      </div>

      <p className="mt-2 truncate text-[10px] tracking-[0.18em] text-neutral-600">
        {connected ? "CONNECTED" : "NOT CONNECTED"}
      </p>
    </button>
  );
}
