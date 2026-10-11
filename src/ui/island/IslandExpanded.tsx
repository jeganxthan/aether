import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Bell } from "lucide-react";

import WifiCard from "../../component/wifi/WifiCard";
import MusicStats from "../../component/music/MusicStats";

const WIFI_POLL_MS = 5000;

const BATTERY_POLL_MS = 5000;

const BATTERY_LEVELS = [
    0, 5, 10, 20, 35, 50, 65, 80, 90, 100,
];

function batteryIcon(
    percentage: number,
    charging: boolean,
) {
    const level = BATTERY_LEVELS.reduce(
        (best, candidate) =>
            candidate <= percentage &&
                candidate > best
                ? candidate
                : best,
        0,
    );

    return `/icons/battery/aether-applet-battery-level-${level}${charging ? "-charging" : ""}-symbolic.svg`;
}

interface WifiStatus {
    enabled: boolean;
    connected: boolean;
    ssid: string | null;
    signal: number | null;
}

interface NotificationSummary {
    id: number;
    isRead: boolean;
}

interface IslandExpandedProps {
    onWifiOpen: () => void;

    /** Opens the notification history panel inside the island. */
    onNotificationsOpen?: () => void;

    /*
     * Tells Island when the music player is on screen so it can size the
     * window for the single-column layout and set Mochi's listening state.
     */
    onMusicVisibleChange?: (
        visible: boolean
    ) => void;
}

interface BatteryStatus {
    percentage: number;
    state:
    | "charging"
    | "discharging"
    | "full"
    | "unknown";
    charging: boolean;
    plugged_in: boolean;
}

interface MediaInfo {
    player: string;
    title: string | null;
    artist: string | null;
    album: string | null;
    art_url: string | null;

    duration: number;
    position: number;

    playback_status: string;

    can_play: boolean;
    can_pause: boolean;
    can_next: boolean;
    can_previous: boolean;
    can_seek: boolean;
}


export default function IslandExpanded({
    onWifiOpen,
    onNotificationsOpen,
    onMusicVisibleChange,
}: IslandExpandedProps) {

    const [wifi, setWifi] =
        useState<WifiStatus>({
            enabled: true,
            connected: false,
            ssid: null,
            signal: null,
        });


    const [battery, setBattery] =
        useState<BatteryStatus>({
            percentage: 0,
            state: "unknown",
            charging: false,
            plugged_in: false,
        });

    const [batteryReady, setBatteryReady] =
        useState(false);


    /*
     * Current MPRIS media.
     */
    const [media, setMedia] =
        useState<MediaInfo | null>(null);


    /*
     * When true, the normal expanded
     * component is temporarily forced.
     */
    const [showDefault, setShowDefault] =
        useState(false);


    /*
     * Load Wi-Fi.
     */
    useEffect(() => {
        let cancelled = false;
        let timer: number | undefined;

        const loadWifi = async () => {
            try {
                const status =
                    await invoke<WifiStatus>(
                        "wifi_status",
                    );

                if (cancelled) {
                    return;
                }

                setWifi(status);
            } catch (error) {
                console.error(
                    "Failed to get WiFi status:",
                    error,
                );
            } finally {
                if (!cancelled) {
                    timer = window.setTimeout(
                        loadWifi,
                        WIFI_POLL_MS,
                    );
                }
            }
        };

        loadWifi();

        return () => {
            cancelled = true;
            window.clearTimeout(timer);
        };
    }, []);


    /*
     * Load battery.
     */
    useEffect(() => {
        let cancelled = false;
        let timer: number | undefined;

        const loadBattery = async () => {
            try {
                const result =
                    await invoke<BatteryStatus>(
                        "battery_status",
                    );

                if (cancelled) {
                    return;
                }

                setBattery(result);
                setBatteryReady(true);
            } catch (error) {
                console.error(
                    "Failed to get battery status:",
                    error,
                );
            } finally {
                if (!cancelled) {
                    timer = window.setTimeout(
                        loadBattery,
                        BATTERY_POLL_MS,
                    );
                }
            }
        };

        loadBattery();

        return () => {
            cancelled = true;
            window.clearTimeout(timer);
        };
    }, []);


    /*
     * Listen to Rust MPRIS service.
     */
    useEffect(() => {
        let unlisten:
            | (() => void)
            | undefined;

        const setup = async () => {
            unlisten =
                await listen<MediaInfo | null>(
                    "media:update",
                    (event) => {
                        setMedia(
                            event.payload,
                        );
                    },
                );

            /*
             * The MPRIS discovery emit can fire before
             * this listener exists (player already
             * running at app start), so bootstrap from
             * the service's cached state on mount.
             */
            const current =
                await invoke<MediaInfo | null>(
                    "media_status",
                );

            setMedia((previous) =>
                previous === null
                    ? current
                    : previous,
            );
        };

        setup();

        return () => {
            unlisten?.();
        };
    }, []);


    /*
     * Unread notifications, shown as a badge on the bell card. The panel
     * unmounts this component while it is open, so closing the panel
     * remounts and reloads the true unread count.
     */
    const [unreadCount, setUnreadCount] =
        useState(0);

    useEffect(() => {
        let cancelled = false;
        let unlistenNew: (() => void) | undefined;
        let unlistenClosed: (() => void) | undefined;
        let unlistenClear: (() => void) | undefined;
        let disposed = false;

        const load = async () => {
            try {
                const history =
                    await invoke<NotificationSummary[]>(
                        "notification_history",
                        { limit: 100 },
                    );

                if (!cancelled) {
                    setUnreadCount(
                        history.filter(
                            (item) => !item.isRead,
                        ).length,
                    );
                }
            } catch (error) {
                console.error(
                    "Failed to load notifications:",
                    error,
                );
            }
        };

        load();

        const setup = async () => {
            const stopNew = await listen(
                "notification:new",
                () => {
                    setUnreadCount((count) =>
                        count + 1,
                    );
                },
            );

            const stopClosed = await listen(
                "notification:closed",
                () => {
                    setUnreadCount((count) =>
                        Math.max(0, count - 1),
                    );
                },
            );

            const stopClear = await listen(
                "notification:clear",
                () => setUnreadCount(0),
            );

            if (disposed) {
                stopNew();
                stopClosed();
                stopClear();
            } else {
                unlistenNew = stopNew;
                unlistenClosed = stopClosed;
                unlistenClear = stopClear;
            }
        };

        setup();

        return () => {
            cancelled = true;
            disposed = true;
            unlistenNew?.();
            unlistenClosed?.();
            unlistenClear?.();
        };
    }, []);


    /*
     * Automatically return to music after
     * the temporary default view expires.
     *
     * If music stopped during those 3 seconds,
     * we simply stay on the default component.
     */
    useEffect(() => {
        if (!showDefault) {
            return;
        }

        const timer =
            window.setTimeout(() => {
                setShowDefault(false);
            }, 3000);

        return () => {
            window.clearTimeout(timer);
        };
    }, [showDefault]);


    /*
     * X button:
     *
     * Show normal expanded component for 3 sec.
     */
    const handleMusicClose = () => {
        setShowDefault(true);
    };


    /*
     * Paused counts too: the row (and its play/pause button)
     * must survive a pause so playback can be resumed from the
     * widget instead of the row unmounting and vanishing.
     */
    const musicPlaying =
        media?.playback_status === "Playing" ||
        media?.playback_status === "Paused";


    /*
     * Music is shown only while:
     *
     * 1. MPRIS reports Playing or Paused
     * 2. User hasn't temporarily dismissed it
     */
    const showMusic =
        musicPlaying &&
        !showDefault;


    /*
     * Let Island react to the music view: taller window,
     * Mochi's listening state.
     */
    useEffect(() => {
        onMusicVisibleChange?.(showMusic);

        return () => {
            onMusicVisibleChange?.(false);
        };
    }, [showMusic, onMusicVisibleChange]);


    return (
        <div className="relative">

            {/* 
             * IMPORTANT:
             *
             * This component intentionally does NOT contain
             * Mochi.
             *
             * Keep Mochi mounted in Island.tsx so changing
             * between these two views doesn't recreate Mochi.
             */}

            {showMusic ? (
                <MusicStats
                    media={media}
                    onClose={
                        handleMusicClose
                    }
                />
            ) : (
                <div
                    className="
                        flex
                        h-[120px]
                        flex-col
                        justify-end
                        pb-5
                    "
                >
                    <div
                        className="
                            border-t
                            border-neutral-800
                            pt-2
                        "
                    >
                        <div className="flex gap-2">
                            <WifiCard
                                enabled={
                                    wifi.enabled
                                }
                                connected={
                                    wifi.connected
                                }
                                ssid={
                                    wifi.ssid
                                }
                                signal={
                                    wifi.signal
                                }
                                onClick={
                                    onWifiOpen
                                }
                            />

                            <div
                                className="
                                    min-w-0
                                    flex-1
                                    rounded-xl
                                    border
                                    border-neutral-900
                                    bg-neutral-950
                                    px-3
                                    py-3
                                "
                            >
                                <div
                                    className="
                                        flex
                                        items-center
                                        gap-2
                                    "
                                >
                                    <img
                                        src="/icons/bluetooth/aether-applet-bluetooth-active-symbolic.svg"
                                        alt=""
                                        className="
                                            h-4
                                            w-4
                                            shrink-0
                                            opacity-70
                                        "
                                    />

                                    <p
                                        className="
                                            truncate
                                            text-xs
                                            text-neutral-300
                                        "
                                    >
                                        Connected
                                    </p>
                                </div>

                                <p
                                    className="
                                        mt-2
                                        truncate
                                        text-[10px]
                                        tracking-[0.18em]
                                        text-neutral-600
                                    "
                                >
                                    GALAXY BUDS2
                                </p>
                            </div>

                            <div
                                className={`
                                    flex
                                    flex-col
                                    shrink-0
                                    rounded-xl
                                    border
                                    border-neutral-900
                                    bg-neutral-950
                                    px-3
                                    py-3
                                    ${battery.charging
                                        ? "w-[88px] justify-start"
                                        : "w-[78px] justify-center"
                                    }
                                `}
                            >
                                {batteryReady && (
                                    <>
                                        <div
                                            className="
                                                flex
                                                items-center
                                                gap-2
                                            "
                                        >
                                            <img
                                                src={batteryIcon(
                                                    battery.percentage,
                                                    battery.charging,
                                                )}
                                                alt=""
                                                className="
                                                    h-4
                                                    w-4
                                                    shrink-0
                                                "
                                            />

                                            <p
                                                className="
                                                    text-xs
                                                    text-neutral-300
                                                "
                                            >
                                                {
                                                    battery.percentage
                                                }
                                                %
                                            </p>
                                        </div>

                                        {battery.charging && (
                                            <p
                                                className="
                                                    mt-2
                                                    text-[10px]
                                                    tracking-[0.18em]
                                                    text-[#30D158]
                                                "
                                            >
                                                CHARGING
                                            </p>
                                        )}
                                    </>
                                )}
                            </div>

                            <button
                                type="button"
                                onClick={onNotificationsOpen}
                                title="Notifications"
                                className="
                                    relative
                                    flex
                                    w-11
                                    shrink-0
                                    items-center
                                    justify-center
                                    rounded-xl
                                    border
                                    border-neutral-900
                                    bg-neutral-950
                                    transition-colors
                                    duration-200
                                    hover:border-neutral-800
                                    hover:bg-neutral-900
                                "
                            >
                                <Bell
                                    size={16}
                                    className="
                                        text-neutral-400
                                    "
                                />

                                {unreadCount > 0 && (
                                    <span
                                        className="
                                            absolute
                                            right-1.5
                                            top-1.5
                                            flex
                                            h-[14px]
                                            min-w-[14px]
                                            items-center
                                            justify-center
                                            rounded-full
                                            bg-[#0A84FF]
                                            px-[3px]
                                            text-[9px]
                                            font-semibold
                                            leading-none
                                            text-white
                                        "
                                    >
                                        {unreadCount > 9
                                            ? "9+"
                                            : unreadCount}
                                    </span>
                                )}
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}