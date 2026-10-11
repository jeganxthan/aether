import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Music2, Pause, Play, SkipBack, SkipForward } from "lucide-react";

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

interface MusicStatsProps {
    onClose: () => void;

    /*
     * Owned by IslandExpanded.
     *
     * This component mounts *because of* a media:update
     * event, so subscribing to that event here would
     * always miss the one that created it and leave the
     * player blank until the next unrelated media event.
     */
    media: MediaInfo | null;
}

export default function MusicStats({
    onClose,
    media,
}: MusicStatsProps) {
    const [position, setPosition] =
        useState(() => media?.position ?? 0);

    const trackRef =
        useRef<HTMLDivElement | null>(
            null,
        );

    /*
     * The bar's own monotonic clock. Position is derived from
     * (syncPos + elapsed-since-sync) so it keeps advancing no
     * matter what the bus/ticker does, and media:update events
     * only ever re-sync it forward — they can never pull it back
     * to zero. syncedAt/title detect track changes and seeks.
     */
    const syncRef = useRef<{
        pos: number;
        at: number;
        title: string | null;
    } | null>(null);

    const rafRef =
        useRef<number | null>(null);

    /*
     * Adopt a Rust-reported position (track change, seek,
     * pause/resume). Re-syncs the wall clock forward; backward
     * jumps that are small are ignored so a laggy ticker can't
     * reset the bar.
     */
    const adoptSync = (
        media: MediaInfo,
    ) => {
        const now = performance.now();
        const previous = syncRef.current;

        /*
         * While Playing, ignore bus reports that sit more than 2s
         * behind the bar's own clock (a laggy or zeroed tick) so
         * they can never yank the bar back toward the start. When
         * not Playing, always adopt the exact reported position so
         * the bar freezes precisely where the song paused.
         */
        const behindBy =
            previous &&
                previous.title === media.title
                ? previous.pos +
                (now - previous.at) *
                1000 -
                media.position
                : 0;

        if (
            media.playback_status === "Playing" &&
            behindBy > 2_000_000
        ) {
            return;
        }

        syncRef.current = {
            pos: media.position,
            at: now,
            title: media.title,
        };

        setPosition(media.position);
    };

    useEffect(() => {
        if (media) {
            adoptSync(media);
        }
    }, [media]);

    /*
     * Album art: MPRIS gives a file:// path, which the asset
     * protocol isn't configured to serve, so Rust reads the file
     * and we build a blob URL. Remote URLs render directly.
     *
     * Rust refreshes the active player every second while
     * playing, so the effect is keyed to track identity
     * (art_url + title) rather than the media object itself,
     * otherwise the art would be re-fetched every second.
     */
    const [cover, setCover] = useState<string | null>(null);
    useEffect(() => {
        let alive = true;
        const url = media?.art_url ?? null;
        setCover(null);
        if (!url) return;
        if (/^https?:/i.test(url)) {
            setCover(url);
            return;
        }
        if (!url.startsWith("file://")) return;
        invoke<[string, number[]]>("read_art", { url })
            .then(([mime, bytes]) => {
                if (!alive) return;
                const blob = new Blob(
                    [new Uint8Array(bytes)],
                    { type: mime },
                );
                setCover(URL.createObjectURL(blob));
            })
            .catch(() => {
                if (alive) setCover(null);
            });
        return () => {
            alive = false;
        };
    }, [media?.art_url, media?.title]);


    /*
     * Advance the bar off its own wall clock while playing.
     *
     * The loop is keyed to playback_status only (not the media
     * object), so a per-second media:update never rebases or
     * restarts it — position is always (sync.pos + elapsed since
     * sync) and it simply keeps going even if the Rust ticker
     * goes silent. rAF is the smooth driver; a 500ms interval is
     * the fallback for WebKit's throttled-rAF windows.
     */
    useEffect(() => {
        if (!media) return;

        if (
            media.playback_status !== "Playing"
        ) {
            if (rafRef.current !== null) {
                cancelAnimationFrame(rafRef.current);
                rafRef.current = null;
            }
            return;
        }

        const tick = () => {
            const sync = syncRef.current;
            if (!sync) return;

            // A 0 duration (player omitted mpris:length) must not
            // clamp the bar to the start.
            const ceiling =
                media.duration > 0
                    ? media.duration
                    : Number.MAX_SAFE_INTEGER;

            setPosition(
                Math.min(
                    sync.pos +
                    (performance.now() -
                        sync.at) *
                    1000,
                    ceiling,
                ),
            );

            rafRef.current =
                requestAnimationFrame(tick);
        };

        tick();

        const interval =
            window.setInterval(tick, 500);

        return () => {
            if (rafRef.current !== null) {
                cancelAnimationFrame(rafRef.current);
                rafRef.current = null;
            }

            window.clearInterval(interval);
        };
    }, [
        media?.playback_status,
        media?.duration,
    ]);


    /*
     * The row stays on screen while playing or paused so the
     * play/pause control can actually toggle back and forth.
     */
    if (
        !media ||
        (media.playback_status !== "Playing" &&
            media.playback_status !== "Paused")
    ) {
        return null;
    }


    const formatTime = (
        microseconds: number,
    ) => {
        const totalSeconds =
            Math.floor(
                microseconds / 1_000_000,
            );

        const minutes =
            Math.floor(
                totalSeconds / 60,
            );

        const seconds =
            totalSeconds % 60;

        return `${minutes}:${seconds
            .toString()
            .padStart(2, "0")}`;
    };


    const handlePlayPause = async () => {
        try {
            await invoke(
                "media_play_pause",
            );
        } catch (error) {
            console.error(
                "Media play/pause failed:",
                error,
            );
        }
    };


    const handlePrevious = async () => {
        try {
            await invoke(
                "media_previous",
            );
        } catch (error) {
            console.error(
                "Previous failed:",
                error,
            );
        }
    };


    const handleNext = async () => {
        try {
            await invoke(
                "media_next",
            );
        } catch (error) {
            console.error(
                "Next failed:",
                error,
            );
        }
    };


    /*
     * Seek by pointer on the custom div bar. Uses media.position
     * (the latest Rust-reported value) for the relative offset so a
     * drag never drifts, unlike the state the rAF loop owns.
     */
    const seekFromClientX = (
        clientX: number,
    ) => {
        const track = trackRef.current;
        if (!track || !media || !media.duration) {
            return;
        }
        if (!media.can_seek) return;

        const rect =
            track.getBoundingClientRect();

        const ratio = Math.min(
            1,
            Math.max(
                0,
                (clientX - rect.left) / rect.width,
            ),
        );

        const newPosition =
            ratio * media.duration;

        setPosition(newPosition);

        syncRef.current = {
            pos: newPosition,
            at: performance.now(),
            title: media.title,
        };

        const offset =
            newPosition - media.position;

        invoke("media_seek", {
            offset: Math.round(offset),
        }).catch(() => {
            console.error("Seek failed");
        });
    };

    const handleTrackDown = (
        event: React.PointerEvent<HTMLDivElement>,
    ) => {
        event.currentTarget.setPointerCapture(
            event.pointerId,
        );
        seekFromClientX(event.clientX);
    };

    const handleTrackMove = (
        event: React.PointerEvent<HTMLDivElement>,
    ) => {
        if (event.buttons & 1) {
            seekFromClientX(event.clientX);
        }
    };


    const progressPct =
        media.duration > 0
            ? (Math.min(position, media.duration) /
                media.duration) *
            100
            : 0;


    return (
        <div className="relative px-4 pb-3">
            {/* Close */}
            <button
                type="button"
                onClick={onClose}
                className="
                absolute
                right-2
                top-1
                z-20
                flex
                h-5
                w-5
                items-center
                justify-center
                rounded-full
                text-neutral-600
                transition
                hover:bg-neutral-900
                hover:text-neutral-300
            "
                aria-label="Show default controls"
            >
                <span className="text-sm">×</span>
            </button>

            {/* Single horizontal music row */}
            <div
                className="
                flex
                w-full
                items-center
                gap-3
                border-neutral-800
                pt-1
            "
            >
                {/* Album cover */}
                <div
                    className="
                    flex
                    h-24
                    w-24
                    shrink-0
                    items-center
                    justify-center
                    self-center
                    overflow-hidden
                    rounded-xl
                    bg-neutral-900
                    ring-1
                    ring-neutral-800
                    text-neutral-500
                "
                >
                    {cover ? (
                        <img
                            src={cover}
                            alt=""
                            className="h-full w-full object-cover"
                            draggable={false}
                        />
                    ) : (
                        <Music2 size={30} />
                    )}
                </div>

                {/* Song information + progress */}
                <div className="min-w-0 flex-1">
                    <div className="flex items-center justify-between gap-2 pr-3">
                        <div className="min-w-0">
                            <p
                                className="
                                truncate
                                text-[17px]
                                font-medium
                                leading-tight
                                text-neutral-100
                            "
                            >
                                {media.title || "Unknown title"}
                            </p>

                            <p
                                className="
                                truncate
                                text-[13px]
                                text-neutral-500
                            "
                            >
                                {media.artist ||
                                    media.album ||
                                    media.player}
                            </p>
                        </div>

                    </div>

                    {/* Progress */}
                    <div className="mt-3 pr-3">
                        <div
                            ref={trackRef}
                            onPointerDown={
                                handleTrackDown
                            }
                            onPointerMove={
                                handleTrackMove
                            }
                            className={`
                            relative
                            w-full
                            cursor-pointer
                            py-[6px]
                            -my-[6px]
                            ${media.can_seek ? "" : "cursor-default"}
                        `}
                        >
                            <div
                                className="
                                relative
                                h-[4px]
                                w-full
                                overflow-hidden
                                rounded-full
                                bg-neutral-800
                            "
                            >
                                <div
                                    className="absolute inset-y-0 left-0 rounded-full bg-neutral-200"
                                    style={{
                                        width: `${progressPct}%`,
                                    }}
                                />
                            </div>
                        </div>

                        <div
                            className="
                            mt-2
                            flex
                            justify-between
                            text-[11px]
                            tabular-nums
                            text-neutral-500
                        "
                        >
                            <span>
                                {formatTime(position)}
                            </span>
                            <span>
                                {formatTime(media.duration)}
                            </span>
                        </div>
                    </div>
                </div>

                {/* Controls */}
                <div
                    className="
                    flex
                    shrink-0
                    items-center
                    gap-4
                    pr-2
                "
                >
                    <button
                        type="button"
                        onClick={handlePrevious}
                        disabled={!media.can_previous}
                        className="
                        text-neutral-400
                        transition
                        hover:text-neutral-100
                        disabled:opacity-20
                    "
                    >
                        <SkipBack size={18} />
                    </button>

                    <button
                        type="button"
                        onClick={handlePlayPause}
                        className="
                        flex
                        h-10
                        w-10
                        items-center
                        justify-center
                        rounded-full
                        bg-neutral-200
                        text-black
                        transition
                        hover:bg-white
                    "
                    >
                        {media.playback_status ===
                            "Playing" ? (
                            <Pause
                                size={17}
                                fill="currentColor"
                            />
                        ) : (
                            <Play
                                size={17}
                                fill="currentColor"
                                className="ml-0.5"
                            />
                        )}
                    </button>

                    <button
                        type="button"
                        onClick={handleNext}
                        disabled={!media.can_next}
                        className="
                        text-neutral-400
                        transition
                        hover:text-neutral-100
                        disabled:opacity-20
                    "
                    >
                        <SkipForward size={18} />
                    </button>
                </div>
            </div>
        </div>
    );
}