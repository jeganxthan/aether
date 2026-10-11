import { useEffect, useState } from "react";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { BellOff, X } from "lucide-react";

export type NotificationLevel = "critical" | "warning" | "normal";

export interface Notification {
    id: number;
    appName: string;
    appIcon: string;
    summary: string;
    body: string;
    level: NotificationLevel;
    /** Unix seconds. */
    createdAt: number;
    isRead: boolean;
    expireTimeout: number;
}

interface NotificationPanelProps {
    /** Closes the panel and collapses the island. */
    onClose: () => void;
}

const MAX_ITEMS = 100;

const LEVEL_COLOR: Record<NotificationLevel, string> = {
    critical: "#FF453A",
    warning: "#FFD60A",
    normal: "#0A84FF",
};

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

export default function NotificationPanel({
    onClose,
}: NotificationPanelProps) {
    const [items, setItems] = useState<Notification[]>([]);

    useEffect(() => {
        invoke<Notification[]>("notification_history", {
            limit: MAX_ITEMS,
        })
            .then(setItems)
            .catch((error) =>
                console.error(
                    "Failed to load notifications:",
                    error
                )
            );
    }, []);

    /*
     * The backend emits one event per change rather than the whole list,
     * so the panel folds them into its local copy: a new notification is
     * prepended (or replaces an existing one by id), a close removes it,
     * and a clear empties the list.
     */
    useEffect(() => {
        let unlistenNew: (() => void) | undefined;
        let unlistenClosed: (() => void) | undefined;
        let unlistenClear: (() => void) | undefined;
        let disposed = false;

        const setup = async () => {
            const stopNew = await listen<Notification>(
                "notification:new",
                (event) => {
                    const incoming = event.payload;
                    setItems((previous) => {
                        const without = previous.filter(
                            (item) => item.id !== incoming.id
                        );
                        return [incoming, ...without].slice(
                            0,
                            MAX_ITEMS
                        );
                    });
                }
            );

            const stopClosed = await listen<number>(
                "notification:closed",
                (event) => {
                    setItems((previous) =>
                        previous.filter(
                            (item) => item.id !== event.payload
                        )
                    );
                }
            );

            const stopClear = await listen(
                "notification:clear",
                () => setItems([])
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
            disposed = true;
            unlistenNew?.();
            unlistenClosed?.();
            unlistenClear?.();
        };
    }, []);

    const markRead = async (id: number) => {
        setItems((previous) =>
            previous.map((item) =>
                item.id === id
                    ? { ...item, isRead: true }
                    : item
            )
        );

        try {
            await invoke("notification_mark_read", { id });
        } catch (error) {
            console.error(
                "Failed to mark notification read:",
                error
            );
        }
    };

    const dismiss = async (id: number) => {
        setItems((previous) =>
            previous.filter((item) => item.id !== id)
        );

        try {
            await invoke("notification_dismiss", { id });
        } catch (error) {
            console.error(
                "Failed to dismiss notification:",
                error
            );
        }
    };

    const clearAll = async () => {
        setItems([]);

        try {
            await invoke("notification_clear_history");
        } catch (error) {
            console.error(
                "Failed to clear notifications:",
                error
            );
        }
    };

    const unread = items.filter((item) => !item.isRead).length;

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
                <span
                    className="
                        text-[18px]
                        font-light
                        text-neutral-100
                    "
                >
                    Notifications
                </span>

                {unread > 0 && (
                    <span
                        className="
                            flex
                            h-[20px]
                            min-w-[20px]
                            items-center
                            justify-center
                            rounded-full
                            bg-[#0A84FF]
                            px-[6px]
                            text-[11px]
                            font-semibold
                            text-white
                        "
                    >
                        {unread}
                    </span>
                )}

                <div className="ml-auto flex shrink-0 items-center gap-2">
                    <button
                        type="button"
                        onClick={clearAll}
                        disabled={items.length === 0}
                        title="Clear all notifications"
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
                        className="
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
                        "
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
                {items.length === 0 && (
                    <div
                        className="
                            flex
                            h-full
                            flex-col
                            items-center
                            justify-center
                            gap-2
                            text-neutral-600
                        "
                    >
                        <BellOff className="h-5 w-5" />
                        <span className="text-sm">
                            No notifications
                        </span>
                    </div>
                )}

                {items.map((item) => (
                    <div
                        key={item.id}
                        onClick={() => {
                            if (!item.isRead) markRead(item.id);
                        }}
                        className={`
                            group

                            flex
                            cursor-default
                            items-start
                            gap-3

                            rounded-[10px]

                            px-[10px]
                            py-[9px]

                            transition-colors

                            hover:bg-white/[0.06]

                            ${item.isRead
                                ? "bg-transparent"
                                : "bg-white/[0.03]"
                            }
                        `}
                    >
                        <span
                            className="
                                mt-[6px]
                                h-[7px]
                                w-[7px]
                                shrink-0
                                rounded-full
                            "
                            style={{
                                backgroundColor:
                                    LEVEL_COLOR[item.level],
                            }}
                        />

                        <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-2">
                                <span
                                    className="
                                        truncate
                                        text-[11px]
                                        tracking-[0.02em]
                                        text-neutral-500
                                    "
                                >
                                    {item.appName ||
                                        "Notification"}
                                </span>

                                <span
                                    className="
                                        shrink-0
                                        text-[11px]
                                        text-neutral-600
                                    "
                                >
                                    {relativeTime(
                                        item.createdAt
                                    )}
                                </span>
                            </div>

                            {item.summary && (
                                <p
                                    className="
                                        mt-[2px]
                                        truncate
                                        text-[13px]
                                        font-medium
                                        text-neutral-100
                                    "
                                >
                                    {item.summary}
                                </p>
                            )}

                            {item.body && (
                                <p
                                    className="
                                        mt-[1px]
                                        line-clamp-2
                                        break-words
                                        text-[12px]
                                        leading-snug
                                        text-neutral-400
                                    "
                                >
                                    {item.body}
                                </p>
                            )}
                        </div>

                        <button
                            type="button"
                            onClick={(event) => {
                                event.stopPropagation();
                                dismiss(item.id);
                            }}
                            title="Dismiss"
                            className="
                                mt-[2px]
                                flex
                                h-[20px]
                                w-[20px]
                                shrink-0
                                items-center
                                justify-center
                                rounded-full

                                text-neutral-500
                                opacity-0

                                transition-all

                                hover:bg-white/[0.08]
                                hover:text-neutral-200

                                group-hover:opacity-100
                            "
                        >
                            <X className="h-[13px] w-[13px]" />
                        </button>
                    </div>
                ))}
            </div>
        </div>
    );
}
