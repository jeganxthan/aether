import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import IslandExpanded from "./IslandExpanded";
import MochiCanvas from "./mochi/MochiCanvas";
import WifiPanel from "../../component/WifiPanel";

const COLLAPSED_HEIGHT = 32;
const EXPANDED_HEIGHT = 205;
const WIFI_HEIGHT = 260;
const MUSIC_HEIGHT = 176;

export default function Island() {
    const [time, setTime] = useState("--:--");
    const [hovered, setHovered] = useState(false);
    const [wifiMode, setWifiMode] = useState(false);
    const [wifiScanning, setWifiScanning] = useState(false);

    const islandRef = useRef<HTMLDivElement>(null);
    const mochiGroupRef = useRef<HTMLDivElement>(null);
    const [wifiShift, setWifiShift] = useState(0);
    const collapseTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
    const isFirstRender = useRef(true);
    const openWifi = useCallback(() => {
        setHovered(true);
        setTimeout(() => {
            setWifiMode(true);
        }, 200);
    }, []);

    const closeWifi = useCallback(() => {
        setWifiMode(false);
        setWifiScanning(false);
        setTimeout(() => {
            setHovered(false);
        }, 300);
    }, []);

    /*
     * True while the music player is on screen and hovered.
     * Sizes the window taller and makes Mochi listen.
     */
    const [musicVisible, setMusicVisible] =
        useState(false);

    const handleMusicVisibleChange =
        useCallback((visible: boolean) => {
            setMusicVisible(visible);
        }, []);

    const musicMode =
        musicVisible && hovered && !wifiMode;
    // Only the height is passed. The window width is pinned in Rust because
    // Wayland cannot move a window, so changing the width would make the island
    // slide sideways as it opened.
    const fitWindow = useCallback(async (height: number) => {
        await invoke("fit_window", { height });
    }, []);

    // Glide the robo from the centered clock row to the Wi-Fi panel's
    // top-left slot. justify-content snaps, so we animate a translateX.
    useLayoutEffect(() => {
        if (!wifiMode) {
            setWifiShift(0);
            return;
        }

        const group = mochiGroupRef.current;
        const container = islandRef.current;

        if (!group || !container) return;

        setWifiShift(
            16 - (container.offsetWidth - group.offsetWidth) / 2,
        );
    }, [wifiMode]);

    useEffect(() => {
        if (isFirstRender.current) {
            isFirstRender.current = false;
            fitWindow(COLLAPSED_HEIGHT);
            return;
        }

        if (hovered) {
            if (collapseTimer.current) {
                clearTimeout(collapseTimer.current);
                collapseTimer.current = null;
            }

            const targetH =
                wifiMode
                    ? WIFI_HEIGHT
                    : musicMode
                        ? MUSIC_HEIGHT
                        : EXPANDED_HEIGHT;

            // Grow the native window immediately so the expand animation isn't clipped
            fitWindow(targetH);
        } else {
            // Hold the window open for the duration of the collapse transition (500ms)
            // before shrinking, so the shrinking widget is never clipped
            collapseTimer.current = setTimeout(() => {
                collapseTimer.current = null;
                fitWindow(COLLAPSED_HEIGHT);
            }, 500);
        }

        return () => {
            if (collapseTimer.current) {
                clearTimeout(collapseTimer.current);
                collapseTimer.current = null;
            }
        };
    }, [hovered, wifiMode, musicMode, fitWindow]);

    useEffect(() => {
        let unlisten: (() => void) | undefined;

        const setup = async () => {
            const currentTime = await invoke<string>("get_time");

            setTime(currentTime);

            unlisten = await listen<string>(
                "clock:update",
                (event) => {
                    setTime(event.payload);
                }
            );
        };

        setup();

        return () => {
            unlisten?.();
        };
    }, []);

    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === "Escape" && wifiMode) {
                closeWifi();
            }
        };

        window.addEventListener("keydown", handleKeyDown);
        return () => window.removeEventListener("keydown", handleKeyDown);
    }, [wifiMode, closeWifi]);

    return (
        <div className="flex w-full justify-center">
            <div
                ref={islandRef}
                onMouseEnter={() => {
                    if (!wifiMode) setHovered(true);
                }}
                onMouseLeave={() => {
                    if (!wifiMode) setHovered(false);
                }}
                className={`
                    relative
                    shrink-0
                    overflow-hidden

                    bg-black
                    text-white

                    border-x
                    border-b
                    border-neutral-800

                    transition-all
                    duration-500
                    ease-[cubic-bezier(0.16,1,0.3,1)]

                    ${wifiMode
                        ? `
                                w-[420px]
                                h-[260px]
                                rounded-b-[28px]
                            `
                        : musicMode
                            ? `
                                w-[420px]
                                h-[176px]
                                rounded-b-[28px]
                            `
                            : hovered
                                ? `
                                w-[420px]
                                h-[205px]
                                rounded-b-[28px]
                            `
                                : `
                                w-[102px]
                                h-[32px]
                                rounded-b-[12px]
                            `
                    }
                `}
            >
                <div
                    className={`
                        absolute
                        inset-x-0
                        top-0

                        flex
                        items-center
                        justify-center

                        pointer-events-none

                        transition-all
                        duration-500
                        ease-[cubic-bezier(0.16,1,0.3,1)]

                        ${hovered
                            ? "h-[104px]"
                            : "h-[32px]"
                        }
                    `}
                >
                    <div
                        ref={mochiGroupRef}
                        style={{
                            transform: `
        translate(
            ${wifiMode
                                    ? wifiShift
                                    : musicMode
                                        ? -165
                                        : 0
                                }px,
            ${musicMode
                                    ? 82
                                    : 0
                                }px
        )
    `,
                        }}
                        className={`
                            flex
                            items-center

                            transition-transform
                            duration-500
                            ease-[cubic-bezier(0.16,1,0.3,1)]

                            ${hovered
                                ? "gap-1"
                                : "gap-1.5"
                            }
                        `}
                    >
                        <div
                            className="
        relative
        z-20
        flex
        items-center
    "
                        >
                            <MochiCanvas
                                expanded={hovered}
                                searching={wifiScanning}
                                listening={musicMode}
                            />
                        </div>

                        <span
                            className={`
                                font-clock
                                leading-none
                                transition-all
                                duration-500
                                ease-[cubic-bezier(0.16,1,0.3,1)]

                                ${hovered
                                    ? "text-[32px] font-light text-neutral-100"
                                    : "text-[13px] font-bold text-neutral-300 "
                                }

                                ${musicMode
                                    ? "hidden"
                                    : wifiMode
                                        ? "opacity-0"
                                        : "opacity-100"
                                }
                            `}
                        >
                            {time}
                        </span>
                    </div>
                </div>

                {wifiMode && (
                    <WifiPanel
                        onClose={closeWifi}
                        onScanningChange={setWifiScanning}
                    />
                )}

                {!wifiMode && (
                    <div
                        className={`
                            absolute
                            top-[104px]
                            left-0
                            right-0

                            px-5

                            transition-all
                            duration-500
                            ease-[cubic-bezier(0.16,1,0.3,1)]

                            ${hovered
                                ? `
                                        translate-y-0
                                        opacity-100
                                        delay-150
                                    `
                                : `
                                        translate-y-4
                                        opacity-0
                                        delay-0
                                    `
                            }
                        `}
                    >
                        <IslandExpanded
                            onWifiOpen={openWifi}
                            onMusicVisibleChange={
                                handleMusicVisibleChange
                            }
                        />
                    </div>
                )}
            </div>
        </div>
    );
}

