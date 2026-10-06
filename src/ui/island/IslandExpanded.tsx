import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import WifiCard from "../../component/WifiCard";
import MusicStats from "../../component/MusicStats";

interface WifiStatus {
    enabled: boolean;
    connected: boolean;
    ssid: string | null;
    signal: number | null;
}

interface IslandExpandedProps {
    onWifiOpen: () => void;

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
    onMusicVisibleChange,
}: IslandExpandedProps) {

    const [wifi, setWifi] =
        useState<WifiStatus>({
            enabled: false,
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
        const loadWifi = async () => {
            try {
                const status =
                    await invoke<WifiStatus>(
                        "wifi_status",
                    );

                console.log(
                    "Aether WiFi status:",
                    status,
                );

                setWifi(status);
            } catch (error) {
                console.error(
                    "Failed to get WiFi status:",
                    error,
                );
            }
        };

        loadWifi();
    }, []);


    /*
     * Load battery.
     */
    useEffect(() => {
        const loadBattery = async () => {
            try {
                const result =
                    await invoke<BatteryStatus>(
                        "battery_status",
                    );

                setBattery(result);
            } catch (error) {
                console.error(
                    "Failed to get battery status:",
                    error,
                );
            }
        };

        loadBattery();
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


    const musicPlaying =
        media?.playback_status ===
        "Playing";


    /*
     * Music is shown only while:
     *
     * 1. MPRIS reports Playing
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
                <div className="px-5 pb-5">

                    <div
                        className="
                            border-t
                            border-neutral-800
                            pt-2
                        "
                    >

                        <div
                            className="
                                grid
                                grid-cols-3
                                gap-2
                            "
                        >

                            {/* Wi-Fi */}
                            <WifiCard
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


                            {/* Battery */}
                            <div
                                className="
                                    rounded-xl
                                    border
                                    border-neutral-900
                                    bg-neutral-950
                                    p-3
                                "
                            >
                                <p
                                    className="
                                        text-[9px]
                                        tracking-[0.18em]
                                        text-neutral-600
                                    "
                                >
                                    BATTERY
                                </p>

                                <p
                                    className="
                                        mt-2
                                        text-xs
                                        text-neutral-300
                                    "
                                >
                                    {
                                        battery.percentage
                                    }%
                                </p>
                            </div>


                            {/* Audio */}
                            <div
                                className="
                                    rounded-xl
                                    border
                                    border-neutral-900
                                    bg-neutral-950
                                    p-3
                                "
                            >
                                <p
                                    className="
                                        text-[9px]
                                        tracking-[0.18em]
                                        text-neutral-600
                                    "
                                >
                                    AUDIO
                                </p>

                                <p
                                    className="
                                        mt-2
                                        text-xs
                                        text-neutral-300
                                    "
                                >
                                    42%
                                </p>
                            </div>

                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}