import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import IslandExpanded from "./IslandExpanded";

export default function Island() {
    const [time, setTime] = useState("--:--");
    const [hovered, setHovered] = useState(false);

    const islandRef = useRef<HTMLDivElement>(null);

    // Keep the native window exactly the size of the rendered widget. Measuring
    // the real element means the CSS sizes stay the single source of truth and
    // the window can never drift out of sync with them.
    useLayoutEffect(() => {
        const el = islandRef.current;
        if (!el) return;

        const resize = () => {
            const rect = el.getBoundingClientRect();
            invoke("resize_window", {
                width: Math.round(rect.width),
                height: Math.round(rect.height),
            });
        };

        resize();

        const observer = new ResizeObserver(resize);
        observer.observe(el);

        return () => observer.disconnect();
    }, []);

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

    return (
        <div className="flex w-full justify-center">
            <div
                ref={islandRef}
                onMouseEnter={() => setHovered(true)}
                onMouseLeave={() => setHovered(false)}
                className={`
                    relative
                    shrink-0
                    overflow-hidden

                    bg-black
                    text-white

                    border-x
                    border-b
                    border-neutral-800

                    shadow-[0_12px_40px_rgba(0,0,0,0.5)]

                    transition-all
                    duration-500
                    ease-[cubic-bezier(0.16,1,0.3,1)]

                    ${
                        hovered
                            ? `
                                w-[420px]
                                h-[205px]
                                rounded-b-[28px]
                            `
                            : `
                                w-[102px]
                                h-[32px]
                                rounded-b-[16px]
                            `
                    }
                `}
            >
                {/* Time */}
                <div
                    className="
                        absolute
                        inset-x-0
                        top-0

                        h-[32px]

                        flex
                        items-center
                        justify-center

                        pointer-events-none
                    "
                >
                    <span
                        className="
                            text-[13px]
                            font-medium
                            tracking-wide
                            leading-none
                        "
                    >
                        {time}
                    </span>
                </div>

                {/* Expanded content */}
                <div
                    className={`
                        absolute
                        top-[52px]
                        left-0
                        right-0

                        px-5

                        transition-all
                        duration-500
                        ease-[cubic-bezier(0.16,1,0.3,1)]

                        ${
                            hovered
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
                    <IslandExpanded />
                </div>
            </div>
        </div>
    );
}