import { useEffect, useRef, useState } from "react";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Pin, Trash2, X } from "lucide-react";

interface ClipboardEntry {
    id: string;
    content: string;
    contentType: string;
    /** Unix seconds. */
    copiedAt: number;
    pinned: boolean;
}

interface ClipboardPanelProps {
    /** Closes the panel and collapses the island. */
    onClose: () => void;
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

function relativeTime(seconds: number): string {
    const diff = Math.max(
        0,
        Math.floor(Date.now() / 1000) - seconds,
    );

    if (diff < 60) return "just now";

    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;

    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;

    return `${Math.floor(diff / 86400)}d ago`;
}

/** Collapse runs of whitespace so multi-line snippets stay readable. */
function preview(content: string): string {
    return content.replace(/\s+/g, " ").trim();
}

export default function ClipboardPanel({
    onClose,
}: ClipboardPanelProps) {
    const inputRef = useRef<HTMLInputElement>(null);
    const listRef = useRef<HTMLDivElement>(null);

    const [entries, setEntries] = useState<
        ClipboardEntry[]
    >([]);

    const [query, setQuery] = useState("");

    const [selectedIndex, setSelectedIndex] =
        useState(0);

    useEffect(() => {
        const focusInput = () => inputRef.current?.focus();
        focusInput();
        const raf = requestAnimationFrame(focusInput);
        window.addEventListener("focus", focusInput);
        return () => {
            cancelAnimationFrame(raf);
            window.removeEventListener("focus", focusInput);
        };
    }, []);

    useEffect(() => {
        invoke<ClipboardEntry[]>("clipboard_history")
            .then(setEntries)
            .catch((error) =>
                console.error(
                    "Failed to load clipboard history:",
                    error
                )
            );
    }, []);

    /*
     * The backend pushes the whole list on every change (new copy, pin,
     * delete, clear), so the panel always mirrors the watcher.
     */
    useEffect(() => {
        let unlisten: (() => void) | undefined;
        let disposed = false;

        const setup = async () => {
            const stop = await listen<ClipboardEntry[]>(
                "clipboard:update",
                (event) => setEntries(event.payload)
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

    const copy = async (id: string) => {
        try {
            await invoke("clipboard_copy", { id });

            onClose();
        } catch (error) {
            console.error(
                "Failed to copy clipboard entry:",
                error
            );
        }
    };

    const togglePin = async (entry: ClipboardEntry) => {
        try {
            const updated = await invoke<
                ClipboardEntry[]
            >("clipboard_pin", {
                id: entry.id,
                pinned: !entry.pinned,
            });

            setEntries(updated);
        } catch (error) {
            console.error(
                "Failed to pin clipboard entry:",
                error
            );
        }
    };

    const remove = async (id: string) => {
        try {
            const updated = await invoke<
                ClipboardEntry[]
            >("clipboard_delete", { id });

            setEntries(updated);
        } catch (error) {
            console.error(
                "Failed to delete clipboard entry:",
                error
            );
        }
    };

    const clear = async () => {
        try {
            const updated = await invoke<
                ClipboardEntry[]
            >("clipboard_clear");

            setEntries(updated);
        } catch (error) {
            console.error(
                "Failed to clear clipboard history:",
                error
            );
        }
    };

    const sorted = [...entries].sort((a, b) => {
        if (a.pinned !== b.pinned) {
            return a.pinned ? -1 : 1;
        }

        return b.copiedAt - a.copiedAt;
    });

    const needle = query.trim().toLowerCase();

    const filtered = needle
        ? sorted.filter((entry) =>
            entry.content.toLowerCase().includes(needle)
        )
        : sorted;

    /*
     * A fresh filter or a changed history starts the selection back at
     * the top so Enter always acts on the first visible row.
     */
    useEffect(() => {
        setSelectedIndex(0);
    }, [query, entries.length]);

    /*
     * Keep the highlighted row within the scroll viewport as the user
     * walks the list with the arrow keys.
     */
    useEffect(() => {
        const node = listRef.current?.children[selectedIndex];

        if (node instanceof HTMLElement) {
            node.scrollIntoView({ block: "nearest" });
        }
    }, [selectedIndex]);

    /*
     * Arrow keys move the selection and Enter copies it. Handled at the
     * window level so it works whether or not the search field holds
     * focus (the compositor owns initial focus on Wayland).
     */
    useEffect(() => {
        const handleKeyDown = (event: KeyboardEvent) => {
            if (event.key === "ArrowDown") {
                event.preventDefault();

                setSelectedIndex((index) =>
                    Math.min(
                        index + 1,
                        filtered.length - 1
                    )
                );
            } else if (event.key === "ArrowUp") {
                event.preventDefault();

                setSelectedIndex((index) =>
                    Math.max(index - 1, 0)
                );
            } else if (event.key === "Enter") {
                event.preventDefault();

                const entry = filtered[selectedIndex];

                if (entry) {
                    copy(entry.id);
                } else {
                    onClose();
                }
            }
        };

        window.addEventListener("keydown", handleKeyDown);

        return () =>
            window.removeEventListener(
                "keydown",
                handleKeyDown
            );
    }, [filtered, selectedIndex]);

    const hasUnpinned = entries.some(
        (entry) => !entry.pinned
    );

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
            {/* Header — left padding clears the island's Mochi. */}
            <div
                className="
                    flex
                    h-[64px]
                    shrink-0
                    items-center
                    gap-3
                    pl-[101px]
                    pr-4
                "
            >
                <input
                    ref={inputRef}
                    value={query}
                    onChange={(event) =>
                        setQuery(event.target.value)
                    }
                    onKeyDown={(e) => {
                        if (e.key === "Enter") {
                            e.preventDefault();
                            const entry = filtered[selectedIndex];
                            if (entry) {
                                copy(entry.id);
                            } else {
                                onClose();
                            }
                        }
                    }}
                    placeholder="Search clipboard..."
                    className="
                        min-w-0
                        flex-1

                        bg-transparent
                        outline-none

                        text-[18px]
                        text-neutral-100

                        placeholder:text-neutral-600
                    "
                    autoComplete="off"
                    spellCheck={false}
                />

                <div className="flex shrink-0 items-center gap-2">
                    <button
                        type="button"
                        onClick={clear}
                        disabled={!hasUnpinned}
                        title="Clear unpinned history"
                        className="
                            h-[24px]
                            rounded-full
                            px-3

                            text-[12px]
                            text-neutral-400

                            transition-colors

                            hover:bg-white/[0.07]
                            hover:text-neutral-200

                            disabled:pointer-events-none
                            disabled:opacity-30
                        "
                    >
                        Clear
                    </button>

                    <button
                        type="button"
                        onClick={onClose}
                        title="Close"
                        className={CONTROL_CLASS}
                    >
                        <X className="h-[15px] w-[15px]" />
                    </button>
                </div>
            </div>

            {/* Divider */}
            <div
                className="
                    h-px
                    w-full
                    shrink-0
                    bg-white/[0.06]
                "
            />

            {/* List */}
            <div
                className="
                    no-scrollbar

                    min-h-0
                    flex-1

                    overflow-y-auto

                    px-[8px]
                    py-[6px]
                "
            >
                {filtered.length === 0 && (
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
                        {entries.length === 0
                            ? "No clipboard history"
                            : "No matches"}
                    </div>
                )}

                {filtered.length > 0 && (
                    <div
                        ref={listRef}
                        className="flex flex-col gap-[2px]"
                    >
                        {filtered.map((entry, index) => (
                            <div
                                key={entry.id}
                                onMouseEnter={() =>
                                    setSelectedIndex(index)
                                }
                                className={`
                                    group

                                    flex
                                    items-start
                                    gap-1

                                    rounded-[10px]

                                    px-[8px]
                                    py-[7px]

                                    transition-colors

                                    ${selectedIndex === index
                                        ? "bg-white/[0.06]"
                                        : "bg-transparent"
                                    }

                                    hover:bg-white/[0.06]
                                `}
                            >
                                <button
                                    type="button"
                                    onClick={() =>
                                        copy(entry.id)
                                    }
                                    title="Copy"
                                    className="
                                        min-w-0
                                        flex-1
                                        text-left
                                    "
                                >
                                    <div
                                        className="
                                            line-clamp-2
                                            break-words

                                            text-[13px]
                                            leading-snug
                                            text-neutral-200
                                        "
                                    >
                                        {preview(
                                            entry.content
                                        )}
                                    </div>

                                    <div
                                        className="
                                            mt-[3px]

                                            text-[11px]
                                            leading-none
                                            text-neutral-500
                                        "
                                    >
                                        {relativeTime(
                                            entry.copiedAt
                                        )}
                                    </div>
                                </button>

                                <div
                                    className="
                                        flex
                                        shrink-0
                                        items-center
                                        gap-1
                                    "
                                >
                                    <button
                                        type="button"
                                        onClick={() =>
                                            togglePin(
                                                entry
                                            )
                                        }
                                        title={
                                            entry.pinned
                                                ? "Unpin"
                                                : "Pin"
                                        }
                                        className={`
                                            ${CONTROL_CLASS}

                                            ${entry.pinned
                                                ? "text-[#4da3ff] opacity-100"
                                                : "opacity-0 group-hover:opacity-100"
                                            }
                                        `}
                                    >
                                        <Pin
                                            className="
                                                h-[14px]
                                                w-[14px]
                                            "
                                        />
                                    </button>

                                    <button
                                        type="button"
                                        onClick={() =>
                                            remove(
                                                entry.id
                                            )
                                        }
                                        title="Delete"
                                        className={`
                                            ${CONTROL_CLASS}

                                            opacity-0
                                            group-hover:opacity-100
                                        `}
                                    >
                                        <Trash2
                                            className="
                                                h-[14px]
                                                w-[14px]
                                            "
                                        />
                                    </button>
                                </div>
                            </div>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}
