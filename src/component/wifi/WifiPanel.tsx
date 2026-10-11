import {
    useEffect,
    useRef,
    useState,
} from "react";

import { invoke } from "@tauri-apps/api/core";
import { CircleChevronRight } from "lucide-react";

interface WifiStatus {
    enabled: boolean;
    connected: boolean;
    ssid: string | null;
    signal: number | null;
}

interface WifiNetwork {
    ssid: string;
    signal: number | null;
    security: string;
    connected: boolean;
}

interface WifiPanelProps {
    onClose: () => void;
    onScanningChange?: (scanning: boolean) => void;
}

function getWifiIcon(
    signal: number | null,
    security: string,
) {
    const secure =
        security &&
        security !== "--" &&
        security.trim() !== "";

    const prefix = secure
        ? "network-wireless-secure-signal"
        : "network-wireless-signal";

    if (
        signal == null ||
        signal <= 0
    ) {
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

const CONTROL_CLASS = `
    flex
    h-[24px]
    w-[24px]

    items-center
    justify-center

    rounded-full

    text-neutral-500

    transition-colors

    hover:bg-white/[0.07]
    hover:text-neutral-200

    disabled:pointer-events-none
    disabled:opacity-30
`;

export default function WifiPanel({
    onClose,
    onScanningChange,
}: WifiPanelProps) {
    const [wifi, setWifi] =
        useState<WifiStatus>({
            enabled: false,
            connected: false,
            ssid: null,
            signal: null,
        });

    const [wifiReady, setWifiReady] =
        useState(false);

    const [networks, setNetworks] =
        useState<WifiNetwork[]>(
            [],
        );

    const [loading, setLoading] =
        useState(false);

    const [toggling, setToggling] =
        useState(false);

    const pendingRef = useRef(false);

    const [selectedNetwork, setSelectedNetwork] =
        useState<WifiNetwork | null>(null);

    const [password, setPassword] =
        useState("");

    const [connecting, setConnecting] =
        useState(false);

    const loadWifi =
        async () => {
            try {
                const status =
                    await invoke<WifiStatus>(
                        "wifi_status",
                    );

                setWifi(status);
                setWifiReady(true);

                return true;
            } catch (error) {
                console.error(
                    "Failed to get Wi-Fi status:",
                    error,
                );

                return false;
            }
        };

    const toggleWifi = async () => {
        try {
            setToggling(true);

            await invoke(
                wifi.enabled
                    ? "wifi_disable"
                    : "wifi_enable",
            );

            await loadWifi();
        } catch (error) {
            console.error(
                "Failed to toggle Wi-Fi:",
                error,
            );
        } finally {
            setToggling(false);
        }
    };

    const fetchNetworks =
        async () =>
            await invoke<
                WifiNetwork[]
            >(
                "wifi_networks",
            );

    const activeSignalOf =
        (
            list: WifiNetwork[],
        ) => {
            const active =
                list.find(
                    (
                        network,
                    ) =>
                        network.connected,
                );

            return active
                ? (active.signal ?? 0)
                : null;
        };

    const scanNetworks =
        async (
            rescan = false,
        ) => {
            if (!wifi.enabled) {
                pendingRef.current = false;
                setNetworks([]);
                onScanningChange?.(false);
                return;
            }

            pendingRef.current = true;

            try {
                setLoading(true);
                onScanningChange?.(true);

                let result =
                    await fetchNetworks();

                setNetworks(result);

                let activeSignal =
                    activeSignalOf(result);

                if (
                    rescan ||
                    activeSignal === 0
                ) {
                    await invoke(
                        "wifi_rescan",
                    );

                    for (
                        let attempt = 0;
                        attempt < 16;
                        attempt++
                    ) {
                        await new Promise(
                            (
                                resolve,
                            ) =>
                                setTimeout(
                                    resolve,
                                    700,
                                ),
                        );

                        if (
                            !pendingRef.current
                        ) {
                            return;
                        }

                        result =
                            await fetchNetworks();

                        setNetworks(result);

                        activeSignal =
                            activeSignalOf(
                                result,
                            );

                        if (
                            activeSignal ===
                            null
                        ) {
                            if (
                                attempt >= 10
                            ) {
                                break;
                            }

                            continue;
                        }

                        if (
                            activeSignal > 0
                        ) {
                            break;
                        }
                    }
                }
            } catch (error) {
                console.error(
                    "Failed to scan Wi-Fi:",
                    error,
                );
            } finally {
                pendingRef.current = false;
                setLoading(false);
                onScanningChange?.(false);
            }
        };

    useEffect(() => {
        let cancelled = false;
        let timer: number | undefined;

        const attempt = async () => {
            const loaded = await loadWifi();

            if (!cancelled && !loaded) {
                timer = window.setTimeout(
                    attempt,
                    1000,
                );
            }
        };

        attempt();

        return () => {
            cancelled = true;
            window.clearTimeout(timer);
        };
    }, []);

    useEffect(() => {
        return () => {
            pendingRef.current = false;
            onScanningChange?.(false);
        };
    }, [onScanningChange]);

    useEffect(() => {
        if (wifi.enabled) {
            scanNetworks();
        } else {
            pendingRef.current = false;
            setNetworks([]);
        }
    }, [wifi.enabled]);

    const sortedNetworks =
        [...networks].sort(
            (a, b) => {
                if (
                    a.connected &&
                    !b.connected
                ) {
                    return -1;
                }

                if (
                    !a.connected &&
                    b.connected
                ) {
                    return 1;
                }

                return (
                    (b.signal ?? 0) -
                    (a.signal ?? 0)
                );
            },
        );

    const connectToNetwork = async (
        network: WifiNetwork,
        networkPassword?: string,
    ) => {
        if (network.connected || connecting) {
            return;
        }

        try {
            setConnecting(true);

            await invoke("wifi_connect", {
                ssid: network.ssid,
                password:
                    networkPassword || null,
            });

            setSelectedNetwork(null);
            setPassword("");

            await new Promise((resolve) =>
                setTimeout(resolve, 700),
            );

            await loadWifi();
            await scanNetworks();
        } catch (error) {
            console.error(
                "Failed to connect to Wi-Fi:",
                error,
            );
        } finally {
            setConnecting(false);
        }
    };
    const handleNetworkClick = (
        network: WifiNetwork,
    ) => {
        if (network.connected || connecting) {
            return;
        }

        const secured =
            network.security &&
            network.security !== "--" &&
            network.security.trim() !== "";

        // Open network → connect immediately
        if (!secured) {
            connectToNetwork(network);
            return;
        }

        // Secured network → open password input
        setSelectedNetwork(network);
        setPassword("");
    };

    return (
        <div
            className="
                absolute
                inset-0
                flex
                flex-col
                overflow-hidden
            "
        >
            {/* =============================================
                HEADER — Wi-Fi controls (the robo is rendered
                by the island so it keeps its animation state)
               ============================================= */}

            <div
                className="
                    flex
                    h-[104px]
                    shrink-0
                    items-center
                    justify-end
                    gap-3
                    px-4
                "
            >
                <div className="flex shrink-0 items-center gap-2">
                    <span
                        className="
                            text-sm
                            font-semibold
                            uppercase
                            tracking-[0.08em]
                            text-neutral-300
                        "
                    >
                        Wi-Fi
                    </span>

                    {/* TOGGLE */}

                    <button
                        type="button"
                        role="switch"
                        aria-checked={wifi.enabled}
                        onClick={toggleWifi}
                        disabled={
                            toggling ||
                            !wifiReady
                        }
                        title={
                            wifi.enabled
                                ? "Turn Wi-Fi off"
                                : "Turn Wi-Fi on"
                        }
                        className={`
                            relative
                            flex
                            h-[18px]
                            w-[32px]
                            shrink-0
                            items-center
                            rounded-full
                            transition-colors
                            disabled:opacity-50
                            ${wifi.enabled
                                ? "bg-[#4da3ff]"
                                : "bg-neutral-700"
                            }
                        `}
                    >
                        <span
                            className={`
                                block
                                h-[14px]
                                w-[14px]
                                rounded-full
                                bg-white
                                shadow
                                transition-transform
                                ${wifi.enabled
                                    ? "translate-x-[16px]"
                                    : "translate-x-[2px]"
                                }
                            `}
                        />
                    </button>

                    {/* REFRESH */}

                    <button
                        type="button"
                        onClick={() =>
                            scanNetworks(
                                true,
                            )
                        }
                        disabled={
                            loading ||
                            !wifi.enabled
                        }
                        title="Refresh networks"
                        className={CONTROL_CLASS}
                    >
                        <svg
                            viewBox="0 0 24 24"
                            className={`
                                h-[14px]
                                w-[14px]

                                ${loading
                                    ? "animate-spin"
                                    : ""
                                }
                            `}
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="2"
                        >
                            <path
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                d="
                                    M20 11a8.1 8.1 0 0 0-15.5-2
                                    M4 5v4h4
                                    M4 13a8.1 8.1 0 0 0 15.5 2
                                    M20 19v-4h-4
                                "
                            />
                        </svg>
                    </button>

                    {/* CLOSE */}

                    <button
                        type="button"
                        onClick={onClose}
                        title="Close"
                        className={CONTROL_CLASS}
                    >
                        <svg
                            viewBox="0 0 24 24"
                            className="
                                h-[15px]
                                w-[15px]
                            "
                            fill="none"
                            stroke="currentColor"
                            strokeWidth="2"
                        >
                            <CircleChevronRight />
                        </svg>
                    </button>
                </div>
            </div>

            {/* DIVIDER */}

            <div
                className="
                    h-px
                    w-full
                    shrink-0
                    bg-white/[0.06]
                "
            />

            {/* =============================================
                DEVICE LIST
               ============================================= */}

            <div
                className="
                    min-h-0
                    flex-1

                    overflow-y-auto

                    px-[10px]
                    py-[6px]

                    no-scrollbar
                "
            >
                {/* WIFI OFF */}

                {wifiReady && !wifi.enabled && (
                    <div
                        className="
                            flex
                            h-full
                            items-center
                            justify-center
                            text-sm
                            text-neutral-500
                        "
                    >
                        Wi-Fi is off
                    </div>
                )}

                {/* NO NETWORKS */}

                {wifi.enabled &&
                    !loading &&
                    sortedNetworks.length ===
                    0 && (
                        <div
                            className="
                                flex
                                h-full
                                items-center
                                justify-center
                                text-sm
                                text-neutral-500
                            "
                        >
                            No networks found
                        </div>
                    )}

                {/* NETWORKS */}

                {wifi.enabled &&
                    sortedNetworks.length >
                    0 && (
                        <div
                            className="
                                flex
                                flex-col
                                gap-[2px]
                            "
                        >
                            {sortedNetworks.map(
                                (network, index) => {
                                    const icon =
                                        getWifiIcon(
                                            network.signal,
                                            network.security,
                                        );

                                    const isSelected =
                                        selectedNetwork?.ssid ===
                                        network.ssid;

                                    return (
                                        <div
                                            key={`${network.ssid}-${index}`}
                                            className="w-full"
                                        >
                                            {/* NETWORK ROW */}

                                            <button
                                                type="button"
                                                disabled={
                                                    network.connected ||
                                                    connecting
                                                }
                                                onClick={() =>
                                                    handleNetworkClick(
                                                        network,
                                                    )
                                                }
                                                className="
                        flex
                        min-h-[40px]
                        w-full
                        items-center
                        gap-[9px]
                        rounded-[9px]
                        px-[8px]
                        text-left
                        transition-colors
                        hover:bg-white/[0.045]
                        disabled:pointer-events-none
                    "
                                            >
                                                {/* WIFI ICON */}

                                                <img
                                                    src={icon}
                                                    alt=""
                                                    className="
                            h-[18px]
                            w-[18px]
                            shrink-0
                        "
                                                />

                                                {/* SSID */}

                                                <div
                                                    className="
                            min-w-0
                            flex-1
                        "
                                                >
                                                    <div
                                                        className={`
                                truncate
                                text-sm
                                font-medium
                                ${network.connected
                                                                ? "text-[#4da3ff]"
                                                                : "text-neutral-100"
                                                            }
                            `}
                                                    >
                                                        {network.ssid}
                                                    </div>

                                                    {network.connected && (
                                                        <div
                                                            className="
                                    mt-[3px]
                                    text-[12px]
                                    leading-none
                                    text-neutral-500
                                "
                                                        >
                                                            Connected
                                                        </div>
                                                    )}
                                                </div>

                                                {/* SIGNAL */}

                                                <span
                                                    className="
                            w-[36px]
                            shrink-0
                            text-right
                            text-xs
                            tabular-nums
                            text-neutral-500
                        "
                                                >
                                                    {network.signal}%
                                                </span>

                                                {/* LOCK */}

                                                {network.security &&
                                                    network.security !==
                                                    "--" && (
                                                        <svg
                                                            viewBox="0 0 24 24"
                                                            className="
                                    h-[13px]
                                    w-[13px]
                                    shrink-0
                                    text-neutral-400
                                "
                                                            fill="currentColor"
                                                        >
                                                            <path
                                                                d="
                                        M17 8h-1V6
                                        a4 4 0 0 0-8 0
                                        v2H7a2 2 0 0 0-2 2
                                        v8a2 2 0 0 0 2 2h10
                                        a2 2 0 0 0 2-2v-8
                                        a2 2 0 0 0-2-2Zm-7-2
                                        a2 2 0 0 1 4 0v2h-4V6Zm7
                                        12H7v-8h10v8Z
                                    "
                                                            />
                                                        </svg>
                                                    )}
                                            </button>

                                            {/* PASSWORD INPUT */}

                                            {isSelected && (
                                                <div
                                                    className="
                            flex
                            items-center
                            gap-2
                            px-[8px]
                            pb-[7px]
                        "
                                                >
                                                    <input
                                                        autoFocus
                                                        type="password"
                                                        value={password}
                                                        onChange={(event) =>
                                                            setPassword(
                                                                event.target.value,
                                                            )
                                                        }
                                                        onKeyDown={(event) => {
                                                            if (
                                                                event.key ===
                                                                "Enter"
                                                            ) {
                                                                if (
                                                                    password.trim()
                                                                ) {
                                                                    connectToNetwork(
                                                                        network,
                                                                        password,
                                                                    );
                                                                }
                                                            }

                                                            if (
                                                                event.key ===
                                                                "Escape"
                                                            ) {
                                                                setSelectedNetwork(
                                                                    null,
                                                                );
                                                                setPassword("");
                                                            }
                                                        }}
                                                        placeholder="Password"
                                                        disabled={connecting}
                                                        className="
                                h-[32px]
                                min-w-0
                                flex-1

                                rounded-[7px]

                                border
                                border-white/[0.08]

                                bg-white/[0.04]

                                px-3

                                text-xs
                                text-neutral-200

                                outline-none

                                placeholder:text-neutral-600

                                focus:border-[#4da3ff]/50
                                focus:bg-white/[0.06]
                            "
                                                    />

                                                    <button
                                                        type="button"
                                                        disabled={
                                                            !password.trim() ||
                                                            connecting
                                                        }
                                                        onClick={() =>
                                                            connectToNetwork(
                                                                network,
                                                                password,
                                                            )
                                                        }
                                                        className="
                                h-[32px]

                                rounded-[7px]

                                bg-[#4da3ff]

                                px-3

                                text-xs
                                font-medium
                                text-black

                                transition-opacity

                                hover:opacity-90

                                disabled:pointer-events-none
                                disabled:opacity-30
                            "
                                                    >
                                                        {connecting
                                                            ? "Connecting..."
                                                            : "Connect"}
                                                    </button>
                                                </div>
                                            )}
                                        </div>
                                    );
                                },
                            )}
                        </div>
                    )}
            </div>
        </div>
    );
}
