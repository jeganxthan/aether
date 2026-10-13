import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Settings } from "lucide-react";
import IslandExpanded from "./IslandExpanded";
import MochiCanvas from "./mochi/MochiCanvas";
import WifiPanel from "../../component/wifi/WifiPanel";
import MiniCava from "../../component/music/MiniCava";
import AppLauncher from "../../component/launcher/AppLauncher";
import ClipboardPanel from "../../component/clipboard/ClipboardPanel";
import NotificationPanel from "../../component/notifications/NotificationPanel";
import type { Notification } from "../../component/notifications/NotificationPanel";
import NotificationPopup from "../../component/notifications/NotificationPopup";
import SystemFeedback, {
    type FeedbackMode,
    type BrightnessDirection,
} from "../../component/osd/SystemFeedback";

import { systemControls } from "../../services/systemControls";
/*
 * Launcher heights, one entry per visible result row. Each step is a row of
 * 56px on top of the 64px input and ~17px of list chrome (border + padding);
 * the last entry caps the dropdown, after which the list scrolls. Tailwind
 * only picks up complete class names, so they are written out in full.
 */
const LAUNCHER_HEIGHTS = [
    "h-[64px]",
    "h-[137px]",
    "h-[193px]",
    "h-[249px]",
    "h-[305px]",
    "h-[361px]",
    "h-[417px]",
];

const LAUNCHER_PIXEL_HEIGHTS = [64, 137, 193, 249, 305, 361, 417];

function getIslandShape(
    noticeActive: boolean,
    launcherMode: boolean,
    launcherResultCount: number,
    clipboardMode: boolean,
    wifiMode: boolean,
    notificationMode: boolean,
    musicMode: boolean,
    feedbackMode: boolean,
    hovered: boolean,
    showCava: boolean,
) {
    if (launcherMode) {
        const h =
            LAUNCHER_PIXEL_HEIGHTS[
            Math.min(
                launcherResultCount,
                LAUNCHER_PIXEL_HEIGHTS.length - 1,
            )
            ];
        return [{ x: 50, y: 0, width: 460, height: h }];
    }
    if (clipboardMode) {
        return [{ x: 50, y: 0, width: 460, height: 330 }];
    }
    if (wifiMode) {
        return [{ x: 48, y: 0, width: 464, height: 280 }];
    }
    if (notificationMode) {
        return [{ x: 48, y: 0, width: 464, height: 320 }];
    }
    if (musicMode) {
        return [{ x: 0, y: 0, width: 560, height: 144 }];
    }
    /*
     * Volume / brightness OSD: a compact bar that replaces the pill while the
     * value is being adjusted. Matches the container size below (300x64,
     * centred in the 560px window).
     */
    if (feedbackMode) {
        return [{ x: 130, y: 0, width: 300, height: 64 }];
    }
    /*
     * A popup replaces the pill for its whole lifetime, so the window stays
     * one fixed popup size whether or not the pointer is over it — the
     * transparent strip above is never touched by hover.
     */
    if (noticeActive) {
        return [{ x: 48, y: 0, width: 464, height: 108 }];
    }
    if (hovered) {
        return [{ x: 48, y: 0, width: 464, height: 224 }];
    }
    const width = showCava ? 148 : 116;
    return [{ x: 222, y: 0, width, height: 28 }];
}

interface InputRect {
    x: number;
    y: number;
    width: number;
    height: number;
}

interface IslandPointerEvent {
    kind: "enter" | "leave" | "motion";
    x: number;
    y: number;
}

function inRects(x: number, y: number, rects: InputRect[]): boolean {
    return rects.some(
        (r) => x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height,
    );
}

export default function Island() {
    const [time, setTime] = useState("--:--");
    const [hovered, setHovered] = useState(false);
    const [wifiMode, setWifiMode] = useState(false);
    const [wifiScanning, setWifiScanning] = useState(false);
    const islandRef = useRef<HTMLDivElement>(null);
    const mochiGroupRef = useRef<HTMLDivElement>(null);
    const dlog = useCallback((m: string) => {
        invoke("debug_log", { msg: m }).catch(() => { });
    }, []);
    const [wifiShift, setWifiShift] = useState(0);
    const openWifi = useCallback(() => {
        setHovered(true);
        setTimeout(() => {
            setWifiMode(true);
        }, 200);
    }, []);

    const closeWifi = useCallback(() => {
        setWifiMode(false);
        setWifiScanning(false);

        requestAnimationFrame(() => {
            if (!islandRef.current?.matches(":hover")) {
                setHovered(false);
            }
        });
    }, []);
    const [feedbackMode, setFeedbackMode] =
        useState<FeedbackMode>(null);

    const [volume, setVolume] = useState(50);
    const [muted, setMuted] = useState(false);
    const [brightness, setBrightness] = useState(50);

    const [brightnessDirection, setBrightnessDirection] =
        useState<BrightnessDirection>(null);

    // True while the OSD is fading out, so the container keeps its size until
    // the animation finishes (see `feedbackActive`).
    const [feedbackClosing, setFeedbackClosing] = useState(false);
    const feedbackActive = feedbackMode != null;

    const feedbackTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
    const feedbackCloseTimer = useRef<ReturnType<typeof setTimeout> | null>(
        null,
    );

    /*
     * Show the OSD and keep it up while the value is being adjusted — every
     * press restarts the dismiss timer. On dismiss, fade out first so the
     * island can tween back down without a snap.
     */
    const showSystemFeedback = (
        mode: Exclude<FeedbackMode, null>,
        direction: BrightnessDirection = null,
    ) => {
        if (feedbackTimer.current) {
            clearTimeout(feedbackTimer.current);
        }
        if (feedbackCloseTimer.current) {
            clearTimeout(feedbackCloseTimer.current);
        }

        setFeedbackClosing(false);
        setFeedbackMode(mode);
        setBrightnessDirection(direction);

        feedbackTimer.current = setTimeout(() => {
            setFeedbackClosing(true);
            feedbackCloseTimer.current = setTimeout(() => {
                setFeedbackMode(null);
                setBrightnessDirection(null);
                setFeedbackClosing(false);
                feedbackCloseTimer.current = null;
            }, 280);
            feedbackTimer.current = null;
        }, 1600);
    };

    useEffect(() => {
        return () => {
            if (feedbackTimer.current) {
                clearTimeout(feedbackTimer.current);
            }
            if (feedbackCloseTimer.current) {
                clearTimeout(feedbackCloseTimer.current);
            }
        };
    }, []);
    const handleVolumeIncrease = async () => {
        try {
            const state = await systemControls.increaseVolume();

            setVolume(state.percent);
            setMuted(state.muted);
            showSystemFeedback("volume");
        } catch (error) {
            console.error("Volume increase failed:", error);
        }
    };

    const handleVolumeDecrease = async () => {
        try {
            const state = await systemControls.decreaseVolume();

            setVolume(state.percent);
            setMuted(state.muted);
            showSystemFeedback("volume");
        } catch (error) {
            console.error("Volume decrease failed:", error);
        }
    };

    const handleMuteToggle = async () => {
        try {
            const state = await systemControls.toggleMute();

            setVolume(state.percent);
            setMuted(state.muted);
            showSystemFeedback("volume");
        } catch (error) {
            console.error("Mute toggle failed:", error);
        }
    };

    const handleBrightnessIncrease = async () => {
        try {
            const state = await systemControls.increaseBrightness();

            setBrightness(state.percent);
            showSystemFeedback("brightness", "up");
        } catch (error) {
            console.error("Brightness increase failed:", error);
        }
    };

    const handleBrightnessDecrease = async () => {
        try {
            const state = await systemControls.decreaseBrightness();

            setBrightness(state.percent);
            showSystemFeedback("brightness", "down");
        } catch (error) {
            console.error("Brightness decrease failed:", error);
        }
    };

    /*
     * The volume / brightness handlers are rebuilt every render, but the
     * `system:control` listener is installed once. Keep the handlers behind a
     * ref so that one listener always calls the current versions.
     */
    const controlRef = useRef({
        handleVolumeIncrease,
        handleVolumeDecrease,
        handleMuteToggle,
        handleBrightnessIncrease,
        handleBrightnessDecrease,
    });
    controlRef.current = {
        handleVolumeIncrease,
        handleVolumeDecrease,
        handleMuteToggle,
        handleBrightnessIncrease,
        handleBrightnessDecrease,
    };
    /*
     * True while the app launcher is open. It has no window of its own any
     * more — Super+Space emits launcher:toggle into this window and the
     * launcher renders as an overlay, so this is just another layout mode
     * alongside wifiMode and musicMode.
     *
     * It deliberately does not touch `hovered`: the pointer keeps owning
     * that flag, so the island is already in the right collapsed/expanded
     * state the moment the launcher closes.
     */
    const [launcherMode, setLauncherMode] =
        useState(false);

    const [launcherResultCount, setLauncherResultCount] =
        useState(0);

    // Set while the window is losing focus (blur). In that case the user has
    // already selected another window, so we must not yank focus back.
    const suppressFocusRestoreRef = useRef(false);

    const closeLauncher = useCallback(() => {
        setLauncherMode(false);
        setLauncherResultCount(0);
        setHovered(false);
        pointerInsideRef.current = false;
        collapsingRef.current = true;
        if (!suppressFocusRestoreRef.current) {
            invoke("focus_last_window").catch(() => { });
        }
    }, []);

    /*
     * True while the clipboard history is open. Like the launcher it is
     * just another layout mode, opened by Super+C (SIGRTMIN+5 -> the
     * clipboard:toggle event) rather than a window of its own.
     */
    const [clipboardMode, setClipboardMode] =
        useState(false);

    const closeClipboard = useCallback(() => {
        setClipboardMode(false);
        setHovered(false);
        pointerInsideRef.current = false;
        collapsingRef.current = true;
        if (!suppressFocusRestoreRef.current) {
            invoke("focus_last_window").catch(() => { });
        }
    }, []);

    // Mirror the overlay flags into refs so the (empty-deps) toggle listeners
    // can tell whether a signal is *closing* a panel and should restore focus.
    const launcherModeRef = useRef(false);
    const clipboardModeRef = useRef(false);
    useEffect(() => {
        launcherModeRef.current = launcherMode;
    }, [launcherMode]);
    useEffect(() => {
        clipboardModeRef.current = clipboardMode;
    }, [clipboardMode]);

    /*
     * True while the notification history panel is open. Like the clipboard
     * it is just another overlay layout mode, opened from the bell card in
     * IslandExpanded rather than a window of its own.
     */
    const [notificationMode, setNotificationMode] =
        useState(false);

    const closeNotifications = useCallback(() => {
        setNotificationMode(false);
        setHovered(false);
        pointerInsideRef.current = false;
        collapsingRef.current = true;
    }, []);

    const openNotifications = useCallback(() => {
        // The overlay panels share one slot; opening this one dismisses
        // the launcher and clipboard.
        setLauncherMode(false);
        setClipboardMode(false);
        setHovered(true);
        setNotificationMode(true);
    }, []);

    const handleResultCountChange = useCallback(
        (count: number) => {
            setLauncherResultCount(count);
        },
        [],
    );

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
        musicVisible &&
        hovered &&
        !wifiMode &&
        !launcherMode &&
        !clipboardMode &&
        !notificationMode;
    // The height is passed in every state; the width only differs while the
    // music row is open. Both sides anchor to the window's left edge, so the
    // island never slides horizontally as the size changes.

    /*
     * True only while a track is actually playing (isPayload "Playing").
     * Drives the collapsed Mochi listening look and the MiniCava that
     * sits beside the island while it is unhovered.
     */
    const [musicPlaying, setMusicPlaying] =
        useState(false);

    /*
     * True while a player is active at all, playing or paused.
     * Separated from musicPlaying so the cava stays on screen and
     * visibly freezes on pause instead of vanishing.
     */
    const [musicPresent, setMusicPresent] =
        useState(false);
    const date = new Date().toLocaleDateString("en-IN", {
        weekday: "short",
        day: "2-digit",
        month: "short",
    });
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

    /*
     * MiniCava hangs off the island's right edge, outside the island's
     * clipped bounds, only while collapsed with music playing. It is a
     * sibling of the island inside the same auto-sized centering
     * wrapper, positioned at left:100% + 8px, so it rides the island's
     * edge live (including the 560px -> 116px collapse transition)
     * instead of being measured once and left stale.
     */
    const showCava =
        musicPresent &&
        !hovered &&
        !wifiMode &&
        !launcherMode &&
        !clipboardMode &&
        !notificationMode &&
        !feedbackMode;

    /*
     * The launcher sizes itself to its matches, growing a row at a time and
     * scrolling once it hits the cap (LAUNCHER_HEIGHTS above).
     */
    const launcherHeightClass =
        LAUNCHER_HEIGHTS[
        Math.min(
            launcherResultCount,
            LAUNCHER_HEIGHTS.length - 1,
        )
        ];

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
        let unlisten: (() => void) | undefined;

        const setup = async () => {
            const current =
                await invoke<{
                    playback_status: string;
                } | null>("media_status");

            const apply = (
                payload: {
                    playback_status: string;
                } | null,
            ) => {
                setMusicPresent(payload != null);
                setMusicPlaying(
                    payload?.playback_status ===
                    "Playing",
                );
            };

            apply(current);

            unlisten = await listen<{
                playback_status: string;
            } | null>("media:update", (event) => {
                apply(event.payload);
            });
        };

        setup();

        return () => {
            unlisten?.();
        };
    }, []);

    /*
     * Super+Space. The shortcut is registered in Rust and lands here as an
     * event rather than a window show/hide, because the launcher now lives
     * inside the island window.
     */
    useEffect(() => {
        let unlisten: (() => void) | undefined;
        let disposed = false;

        const setup = async () => {
            const stop = await listen(
                "launcher:toggle",
                () => {
                    // Toggling the open launcher closed → hand focus back.
                    if (launcherModeRef.current) {
                        invoke("focus_last_window").catch(() => { });
                    } else {
                        // The two panels share the same overlay slot, so
                        // opening one must dismiss the other.
                        setClipboardMode(false);
                    }

                    setLauncherMode((open) => !open);
                },
            );

            // React StrictMode runs effects twice in development. If this
            // effect was already cleaned up, immediately drop the listener
            // instead of leaving a second one registered — two listeners
            // would toggle twice per event and the launcher would never open.
            if (disposed) {
                stop();
            } else {
                unlisten = stop;
            }
        };

        setup();

        return () => {
            disposed = true;
            unlisten?.();
        };
    }, []);

    /*
     * Super+C. Same StrictMode-safe pattern as launcher:toggle — a leaked
     * second listener would toggle twice per signal.
     */
    useEffect(() => {
        let unlisten: (() => void) | undefined;
        let disposed = false;

        const setup = async () => {
            const stop = await listen(
                "clipboard:toggle",
                () => {
                    // Toggling the open clipboard closed → hand focus back.
                    if (clipboardModeRef.current) {
                        invoke("focus_last_window").catch(() => { });
                    } else {
                        // Mirror launcher:toggle — only one overlay at a time.
                        setLauncherMode(false);
                    }

                    setClipboardMode((open) => !open);
                },
            );

            if (disposed) {
                stop();
            } else {
                unlisten = stop;
            }
        };

        setup();

        return () => {
            disposed = true;
            unlisten?.();
        };
    }, []);

    /*
     * Volume / brightness keys. labwc runs `pkill -RTMIN+n -x aether-tauri`
     * on the media keys; Rust turns each realtime signal into a
     * `system:control` event here. Same StrictMode-safe pattern as
     * launcher:toggle — one leaked listener would double every press.
     */
    useEffect(() => {
        let unlisten: (() => void) | undefined;
        let disposed = false;

        const setup = async () => {
            const stop = await listen<{
                kind: "volume" | "brightness";
                action: "up" | "down" | "mute";
            }>("system:control", (event) => {
                const { kind, action } = event.payload;
                dlog(`system:control kind=${kind} action=${action}`);

                const handlers = controlRef.current;

                if (kind === "volume") {
                    if (action === "up") handlers.handleVolumeIncrease();
                    else if (action === "down") handlers.handleVolumeDecrease();
                    else handlers.handleMuteToggle();
                } else if (action === "up") {
                    handlers.handleBrightnessIncrease();
                } else {
                    handlers.handleBrightnessDecrease();
                }
            });

            if (disposed) {
                stop();
            } else {
                unlisten = stop;
            }
        };

        setup();

        return () => {
            disposed = true;
            unlisten?.();
        };
    }, [dlog]);

    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === "Escape" && wifiMode) {
                closeWifi();
            }

            if (e.key === "Escape" && launcherMode) {
                closeLauncher();
            }

            if (e.key === "Escape" && clipboardMode) {
                closeClipboard();
            }

            if (e.key === "Escape" && notificationMode) {
                closeNotifications();
            }
        };

        window.addEventListener("keydown", handleKeyDown);
        return () =>
            window.removeEventListener("keydown", handleKeyDown);
    }, [
        wifiMode,
        closeWifi,
        launcherMode,
        closeLauncher,
        clipboardMode,
        closeClipboard,
        notificationMode,
        closeNotifications,
    ]);

    /*
     * Live notification popup. The latest `notification:new` replaces the
     * pill for a few seconds; clicking it opens the full panel. Popups are
     * skipped while a panel / the launcher has the island, so they never
     * fight an open overlay.
     */
    const [notice, setNotice] = useState<Notification | null>(
        null,
    );
    const [noticeClosing, setNoticeClosing] = useState(false);
    const noticeTimerRef = useRef<number | undefined>(undefined);
    const noticeRef = useRef<Notification | null>(null);
    const closingRef = useRef(false);
    noticeRef.current = notice;
    const noticeActive = notice != null;
    // While the popup fades out it stops holding the island open, so the
    // container tweens back down at the same time (matching the island's
    // normal close) instead of snapping after the fade finishes.
    const noticePop = noticeActive && !noticeClosing;

    const bigMode =
        launcherMode ||
        clipboardMode ||
        wifiMode ||
        notificationMode ||
        musicMode;
    const bigModeRef = useRef(bigMode);
    bigModeRef.current = bigMode;

    /*
     * Dismiss with an exit animation: fade the popup out, then unmount it so
     * the island can tween back down to the pill. Idempotent, so the
     * auto-dismiss timer, the × button and a window blur can all call it.
     */
    const dismissNotice = useCallback(() => {
        if (closingRef.current || !noticeRef.current) return;
        closingRef.current = true;
        window.clearTimeout(noticeTimerRef.current);
        setNoticeClosing(true);
        noticeTimerRef.current = window.setTimeout(() => {
            closingRef.current = false;
            setNoticeClosing(false);
            setNotice(null);
        }, 500);
    }, []);

    const dismissNoticeRef = useRef(dismissNotice);
    dismissNoticeRef.current = dismissNotice;

    useEffect(() => {
        let unlisten: (() => void) | undefined;
        let disposed = false;

        const setup = async () => {
            const stop = await listen<Notification>(
                "notification:new",
                (event) => {
                    if (bigModeRef.current) return;
                    closingRef.current = false;
                    setNoticeClosing(false);
                    setNotice(event.payload);
                    window.clearTimeout(noticeTimerRef.current);
                    noticeTimerRef.current = window.setTimeout(() => {
                        dismissNoticeRef.current();
                    }, 5000);
                },
            );

            if (disposed) {
                stop();
            } else {
                unlisten = stop;
            }
        };

        setup();

        return () => {
            disposed = true;
            window.clearTimeout(noticeTimerRef.current);
            unlisten?.();
        };
    }, []);

    const isExpanded =
        launcherMode ||
        clipboardMode ||
        wifiMode ||
        notificationMode ||
        musicMode ||
        hovered;

    /*
     * Guard against hover re-entry during the collapse animation.
     * When the island collapses, the input shape is still large for
     * up to 500ms. Without this guard the transparent area between
     * the shrinking island div and the still-large input region can
     * re-trigger onMouseEnter, causing an expand→collapse loop.
     */
    const collapsingRef = useRef(false);

    const mountedRef = useRef(false);

    /*
     * Compositor-authoritative hover, mirrored into a boolean ref the rest of
     * the island reads. WebKitGTK does not reliably deliver a pointerleave
     * (nor clear CSS `:hover`) when the pointer leaves the Wayland surface or
     * when the input region changes, which stranded the island expanded. GDK
     * receives the real
     * crossings straight from compositor and forwards them via lib.rs; the
     * island trusts only those, not the DOM rect.
     */
    const pointerInsideRef = useRef(false);
    const lastPosRef = useRef({ x: -1, y: -1 });
    const lastEnterAtRef = useRef(-Infinity);
    const shapeRef = useRef<InputRect[]>([]);

    /*
     * Hover forwarded from the GDK enter/leave handlers in lib.rs, with the
     * pointer position so transient region-change artifacts are ignored.
     */
    useEffect(() => {
        let disposed = false;
        let unlisten: (() => void) | undefined;
        let leaveTimer: number | undefined;
        /*
         * Resizing the input shape (expand/collapse) makes GTK emit a
         * transient leave at the stale region edge. Debounce the leave so a
         * real re-entry within 100ms keeps the island open.
         */
        const LEAVE_GRACE_MS = 100;
        listen<IslandPointerEvent>("island:pointer", (event) => {
            const p = event.payload;
            lastPosRef.current = { x: p.x, y: p.y };
            if (p.kind === "enter") {
                dlog(`tauri enter ${Math.round(p.x)},${Math.round(p.y)}`);
                lastEnterAtRef.current = performance.now();
                if (inRects(p.x, p.y, shapeRef.current)) {
                    window.clearTimeout(leaveTimer);
                    leaveTimer = undefined;
                    pointerInsideRef.current = true;
                    collapsingRef.current = false;
                    if (!wifiMode && !launcherMode && !clipboardMode && !notificationMode) {
                        setHovered(true);
                    }
                }
            } else if (p.kind === "leave") {
                /*
                 * Growing the input shape on expand makes GTK emit a leave a
                 * few ms later at the new edge, even though the pointer never
                 * moved. That spurious leave collapsed the island the instant
                 * it opened. The artifact always trails the enter by
                 * well under 100ms, while a genuine exit comes seconds later,
                 * so a short window tells them apart without swallowing real
                 * leaves.
                 */
                const dtEnter = performance.now() - lastEnterAtRef.current;
                dlog(`tauri leave ${Math.round(p.x)},${Math.round(p.y)} dtEnter=${Math.round(dtEnter)}`);
                if (dtEnter < 150) {
                    return;
                }
                pointerInsideRef.current = false;
                if (!wifiMode && !launcherMode && !clipboardMode) {
                    window.clearTimeout(leaveTimer);
                    leaveTimer = window.setTimeout(() => {
                        setHovered(false);
                        collapsingRef.current = true;
                    }, LEAVE_GRACE_MS);
                }
            }
        }).then((u) => {
            if (disposed) u();
            else unlisten = u;
        });
        return () => {
            disposed = true;
            window.clearTimeout(leaveTimer);
            unlisten?.();
        };
    }, [wifiMode, launcherMode, clipboardMode, notificationMode, dlog]);

    useEffect(() => {
        const rects = getIslandShape(
            noticePop,
            launcherMode,
            launcherResultCount,
            clipboardMode,
            wifiMode,
            notificationMode,
            musicMode,
            feedbackActive,
            hovered,
            showCava,
        );
        shapeRef.current = rects;

        if (!mountedRef.current) {
            mountedRef.current = true;
            invoke("set_input_shape", { rects }).catch(console.error);
            return;
        }

        dlog(`effect isExpanded=${isExpanded} hovered=${hovered} notice=${noticeActive}`);
        if (isExpanded || noticePop || feedbackActive) {
            invoke("set_input_shape", { rects }).catch(console.error);
        } else {
            // Shrink the input shape after the CSS transition finishes.
            const timer = setTimeout(() => {
                invoke("set_input_shape", { rects }).catch(console.error);
                collapsingRef.current = false;

                // Re-open only if the pointer is genuinely still on the
                // (newly shrunk) island.
                const last = lastPosRef.current;
                if (pointerInsideRef.current || inRects(last.x, last.y, rects)) {
                    setHovered(true);
                } else {
                    setHovered(false);
                }
            }, 520);
            return () => clearTimeout(timer);
        }
    }, [
        noticePop,
        launcherMode,
        launcherResultCount,
        clipboardMode,
        wifiMode,
        notificationMode,
        musicMode,
        feedbackActive,
        hovered,
        showCava,
        isExpanded,
    ]);

    useEffect(() => {
        const handleBlur = () => {
            // Blur means the user already picked another window; suppress the
            // focus-restore so we don't fight that choice.
            suppressFocusRestoreRef.current = true;
            if (launcherMode) closeLauncher();
            if (clipboardMode) closeClipboard();
            if (notificationMode) closeNotifications();
            suppressFocusRestoreRef.current = false;
            dismissNotice();

            // Focus left the island (another window was clicked). Promise
            // the collapsed look so the pill never lingers expanded.
            pointerInsideRef.current = false;
            setHovered(false);
        };
        window.addEventListener("blur", handleBlur);
        return () => window.removeEventListener("blur", handleBlur);
    }, [launcherMode, closeLauncher, clipboardMode, closeClipboard, notificationMode, closeNotifications, dismissNotice]);

    return (
        <div className="relative flex w-full justify-center">
            <div className="relative shrink-0">
                <div
                    ref={islandRef}
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

                    ${launcherMode
                            ? `
                                w-[460px]
                                ${launcherHeightClass}
                                rounded-b-[16px]
                            `
                            : clipboardMode
                                ? `
                                w-[460px]
                                h-[330px]
                                rounded-b-[16px]
                            `
                                : notificationMode
                                    ? `
                                w-[464px]
                                h-[320px]
                                rounded-b-[16px]
                            `
                                    : wifiMode
                                        ? `
                                w-[464px]
                                h-[280px]
                                rounded-b-[16px]
                            `
                                        : musicMode
                                            ? `
                                w-[560px]
                                h-[144px]
                                rounded-b-[16px]
                            `
                                            : feedbackMode
                                                ? `
                                w-[300px]
                                h-[64px]
                                rounded-b-[16px]
                            `
                                                : noticePop
                                                    ? `
                                w-[464px]
                                h-[108px]
                                rounded-b-[16px]
                            `
                                                : hovered
                                                    ? `
                                w-[464px]
                                h-[224px]
                                rounded-b-[16px]
                            `
                                                    : `
                                w-[116px]
                                h-[28px]
                                rounded-b-[10px]
                            `
                        }
                `}
                >
                    {/*
                     * Settings affordance in the expanded island's top-right
                     * corner. Placeholder for now — wiring comes later.
                     */}
                    {hovered &&
                        !launcherMode &&
                        !clipboardMode &&
                        !wifiMode &&
                        !notificationMode &&
                        !musicMode &&
                        !feedbackMode && (
                            <button
                                type="button"
                                title="Settings"
                                aria-label="Settings"
                                className="
                                    absolute
                                    right-3
                                    top-3
                                    z-30

                                    flex
                                    h-7
                                    w-7
                                    items-center
                                    justify-center

                                    rounded-full

                                    text-neutral-400

                                    transition-colors
                                    duration-200

                                    hover:bg-white/[0.07]
                                    hover:text-neutral-200
                                "
                            >
                                <Settings size={16} />
                            </button>
                        )}

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

                        ${feedbackMode ? "opacity-0" : ""}

                        ${launcherMode || clipboardMode
                                ? "h-[64px]"
                                : hovered
                                    ? "h-[116px]"
                                    : "h-[28px]"
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
                                            ? -190
                                            : launcherMode ||
                                                clipboardMode
                                                ? -179
                                                : 0
                                    }px,
                                ${musicMode
                                        ? 17
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
                                    expanded={
                                        hovered ||
                                        launcherMode ||
                                        clipboardMode
                                    }
                                    launcher={
                                        launcherMode ||
                                        clipboardMode
                                    }
                                    searching={wifiScanning}
                                    listening={
                                        hovered
                                            ? musicMode
                                            : musicPlaying
                                    }
                                    music={musicMode}
                                    greet={
                                        !(
                                            launcherMode ||
                                            clipboardMode ||
                                            musicMode
                                        )
                                    }
                                />
                            </div>

                            <div
                                className={`
                                flex
                                flex-col
                                items-center

                                ${musicMode ||
                                        launcherMode ||
                                        clipboardMode
                                        ? "hidden"
                                        : wifiMode
                                            ? "opacity-0"
                                            : "opacity-100"
                                    }
                            `}
                            >
                                <span
                                    className={`
                                font-clock
                                leading-none
                                transition-all
                                duration-500
                                ease-[cubic-bezier(0.16,1,0.3,1)]

                                ${hovered
                                            ? "text-[36px] font-light text-neutral-100"
                                            : "text-[15px] font-bold text-neutral-300"
                                        }
                            `}
                                >
                                    {time}
                                </span>

                                <span
                                    className={`
                                font-clock
                                leading-none
                                mt-0.5
                                text-neutral-500
                                transition-all
                                duration-500
                                ease-[cubic-bezier(0.16,1,0.3,1)]

                                ${hovered
                                            ? "text-[11px]"
                                            : "hidden"
                                        }
                            `}
                                >
                                    {date}
                                </span>
                            </div>


                        </div>
                    </div>

                    {wifiMode && (
                        <WifiPanel
                            onClose={closeWifi}
                            onScanningChange={setWifiScanning}
                        />
                    )}

                    {!wifiMode &&
                        !launcherMode &&
                        !clipboardMode &&
                        !notificationMode &&
                        !feedbackMode &&
                        (musicMode || !noticeActive) && (
                            <div
                                className={`
                            absolute

                            ${musicMode
                                        ? `top-[26px] left-[128px] right-0 pl-0 pr-6`
                                        : `
                                    top-[104px]
                                    left-0
                                    right-0
                                    px-5
                                `
                                    }

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
                                    onNotificationsOpen={openNotifications}
                                    onMusicVisibleChange={
                                        handleMusicVisibleChange
                                    }
                                />
                            </div>
                        )}

                    {noticeActive &&
                        !notificationMode &&
                        !launcherMode &&
                        !clipboardMode &&
                        !wifiMode &&
                        !musicMode &&
                        !feedbackMode && (
                            <NotificationPopup
                                notification={notice}
                                closing={noticeClosing}
                                onClose={dismissNotice}
                                onOpen={openNotifications}
                            />
                        )}

                    {/*
                     * Volume / brightness OSD: a compact centred bar with Mochi
                     * and a value readout. It replaces the pill for a moment
                     * and never takes focus (the media keys are passive).
                     */}
                    {feedbackMode &&
                        !notificationMode &&
                        !launcherMode &&
                        !clipboardMode &&
                        !wifiMode &&
                        !musicMode &&
                        !noticeActive && (
                            <div
                                className={`
                                    absolute
                                    inset-0
                                    z-30

                                    flex
                                    items-center
                                    gap-3
                                    px-4

                                    transition-all
                                    duration-300
                                    ease-[cubic-bezier(0.16,1,0.3,1)]

                                    ${feedbackClosing
                                        ? "translate-y-3 opacity-0"
                                        : "translate-y-0 opacity-100"
                                    }
                                `}
                            >
                                <MochiCanvas
                                    expanded
                                    greet={false}
                                    scale={0.42}
                                />

                                <SystemFeedback
                                    mode={feedbackMode}
                                    volume={volume}
                                    muted={muted}
                                    brightness={brightness}
                                    brightnessDirection={brightnessDirection}
                                />
                            </div>
                        )}

                    {launcherMode && (
                        <AppLauncher
                            onClose={closeLauncher}
                            onResultCountChange={
                                handleResultCountChange
                            }
                        />
                    )}

                    {clipboardMode && (
                        <ClipboardPanel
                            onClose={closeClipboard}
                        />
                    )}

                    {notificationMode && (
                        <NotificationPanel
                            onClose={closeNotifications}
                        />
                    )}
                </div>

                <div
                    className="
                    absolute
                    left-[calc(100%+8px)]
                    top-[2px]
                    z-30
                    pointer-events-none

                    transition-opacity
                    duration-300
                    ease-[cubic-bezier(0.16,1,0.3,1)]
                "
                    style={{
                        opacity: showCava ? 1 : 0,
                    }}
                    aria-hidden
                >
                    <MiniCava
                        playing={
                            musicPlaying && !wifiMode
                        }
                    />
                </div>
            </div>
        </div>
    );
}

