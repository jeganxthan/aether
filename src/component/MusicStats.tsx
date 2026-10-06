import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Pause, SkipBack, SkipForward } from "lucide-react";

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

    const animationRef =
        useRef<number | null>(null);

    /*
     * Adopt positions Rust reports for us
     * (track change, seek, pause/resume).
     */
    useEffect(() => {
        if (media) {
            setPosition(media.position);
        }
    }, [media]);


    /*
     * Smoothly advance the progress bar while
     * the song is playing.
     *
     * We do NOT constantly ask Rust for the position.
     */
    useEffect(() => {
        if (
            !media ||
            media.playback_status !== "Playing"
        ) {
            if (animationRef.current !== null) {
                cancelAnimationFrame(
                    animationRef.current,
                );

                animationRef.current = null;
            }

            return;
        }

        let lastTime =
            performance.now();

        const update = (
            currentTime: number,
        ) => {
            const delta =
                currentTime - lastTime;

            lastTime = currentTime;

            setPosition((current) => {
                const next =
                    current +
                    delta * 1000;

                return Math.min(
                    next,
                    media.duration,
                );
            });

            animationRef.current =
                requestAnimationFrame(update);
        };

        animationRef.current =
            requestAnimationFrame(update);

        return () => {
            if (
                animationRef.current !== null
            ) {
                cancelAnimationFrame(
                    animationRef.current,
                );

                animationRef.current = null;
            }
        };
    }, [
        media?.playback_status,
        media?.duration,
    ]);


    if (
        !media ||
        media.playback_status !== "Playing"
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


    const handleSeek = async (
        event: React.ChangeEvent<HTMLInputElement>,
    ) => {
        const newPosition =
            Number(event.target.value);

        setPosition(newPosition);

        const offset =
            newPosition - position;

        try {
            await invoke(
                "media_seek",
                {
                    offset: Math.round(offset),
                },
            );
        } catch (error) {
            console.error(
                "Seek failed:",
                error,
            );
        }
    };


   return (
    <div className="relative px-4 pb-2">
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
            <span className="text-xs">×</span>
        </button>

        {/* Single horizontal music row */}
        <div
            className="
                flex
                w-full
                items-center
                gap-3
                border-t
                border-neutral-800
                pt-1
            "
        >
            {/* Song information + progress */}
            <div className="min-w-0 flex-1">
                <div className="flex items-center justify-between gap-2 pr-2">
                    <div className="min-w-0">
                        <p
                            className="
                                truncate
                                text-[11px]
                                font-medium
                                text-neutral-200
                            "
                        >
                            {media.title || "Unknown title"}
                        </p>

                        <p
                            className="
                                mt-0.5
                                truncate
                                text-[9px]
                                text-neutral-600
                            "
                        >
                            {media.artist ||
                                media.album ||
                                media.player}
                        </p>
                    </div>

                    {/* Date / time */}
                    <div className="shrink-0 text-right">
                        <p className="text-[8px] text-neutral-600">
                            Sat, Sep 05
                        </p>

                        <p className="text-sm font-medium tabular-nums text-neutral-300">
                            11:28
                        </p>
                    </div>
                </div>

                {/* Progress */}
                <div className="mt-2 pr-2">
                    <input
                        type="range"
                        min={0}
                        max={Math.max(media.duration, 1)}
                        value={Math.min(
                            position,
                            media.duration,
                        )}
                        onChange={handleSeek}
                        className="
                            h-[2px]
                            w-full
                            cursor-pointer
                            appearance-none
                            rounded-full
                            bg-neutral-800
                        "
                    />

                    <div
                        className="
                            mt-0.5
                            flex
                            justify-between
                            text-[7px]
                            tabular-nums
                            text-neutral-700
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
                    gap-3
                    pr-2
                "
            >
                <button
                    type="button"
                    onClick={handlePrevious}
                    disabled={!media.can_previous}
                    className="
                        text-neutral-500
                        transition
                        hover:text-neutral-200
                        disabled:opacity-20
                    "
                >
                    <SkipBack size={13} />
                </button>

                <button
                    type="button"
                    onClick={handlePlayPause}
                    className="
                        flex
                        h-8
                        w-8
                        items-center
                        justify-center
                        rounded-full
                        bg-neutral-200
                        text-black
                        transition
                        hover:bg-white
                    "
                >
                    <Pause size={12} fill="currentColor" />
                </button>

                <button
                    type="button"
                    onClick={handleNext}
                    disabled={!media.can_next}
                    className="
                        text-neutral-500
                        transition
                        hover:text-neutral-200
                        disabled:opacity-20
                    "
                >
                    <SkipForward size={13} />
                </button>
            </div>
        </div>
    </div>
);
}