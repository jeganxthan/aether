interface WifiCardProps {
  connected: boolean;
  ssid?: string | null;
  signal?: number | null;
  security?: string;
  onClick?:()=>void;
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

export default function WifiCard({
  connected,
  ssid,
  signal,
  security,
  onClick
}: WifiCardProps) {
  const wifiIcon = getWifiIcon(signal, security);

  return (
    <button onClick={onClick}>
    <div className="rounded-xl border border-neutral-900 bg-neutral-950 p-3">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <img
            src={wifiIcon}
            alt=""
            className="h-4 w-4 opacity-70"
          />

          <p className="text-[9px] tracking-[0.18em] text-neutral-600">
            NETWORK
          </p>
        </div>

      </div>

      {/* Network */}
      <div className="mt-2">
        <p className="truncate text-xs text-neutral-300">
          {connected
            ? ssid || "Connected"
            : "Not connected"}
        </p>

        {connected &&
          signal != null && (
            <p className="mt-1 text-[10px] text-neutral-600">
              {signal}% signal
            </p>
          )}
      </div>
    </div>
          </button>
  );
}