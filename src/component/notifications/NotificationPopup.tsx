import { useEffect, useState } from "react";
import { X } from "lucide-react";
import MochiCanvas from "../../ui/island/mochi/MochiCanvas";
import type { Notification } from "./NotificationPanel";

interface NotificationPopupProps {
    notification: Notification;
    /** True once dismissal has started; fades the popup out. */
    closing: boolean;
    /** Dismisses the popup (back to the collapsed pill). */
    onClose: () => void;
    /** Opens the full notification history panel. */
    onOpen: () => void;
}

/**
 * Minimal in-island toast. Mochi appears in a look matching the
 * notification's level (see the engine's `setNoticeState`), alongside the
 * app name / summary / body. Clicking dismisses-free text opens the full
 * panel; the × only closes the toast. Auto-dismiss is handled by Island.
 */
export default function NotificationPopup({
    notification,
    closing,
    onClose,
    onOpen,
}: NotificationPopupProps) {
    const name = notification.appName || "Notification";

    // Fade in on mount, then let `closing` fade it back out before Island
    // unmounts the popup and tweens the island down to the pill.
    const [entered, setEntered] = useState(false);
    useEffect(() => {
        const id = requestAnimationFrame(() => setEntered(true));
        return () => cancelAnimationFrame(id);
    }, []);

    const visible = entered && !closing;

    return (
        <div
            className={`
                absolute
                inset-0
                z-40
                flex
                cursor-pointer
                flex-col
                justify-center
                rounded-b-[16px]
                bg-black/90
                px-5
                py-3
                text-left

                transition-all
                duration-500
                ease-[cubic-bezier(0.16,1,0.3,1)]

                ${visible
                    ? "opacity-100 translate-y-0"
                    : "opacity-0 translate-y-4"
                }
                ${closing ? "pointer-events-none" : ""}
            `}
            onClick={onOpen}
        >
            <div className="flex items-center gap-4">
                <MochiCanvas
                    expanded
                    scale={0.72}
                    notice={notification.level}
                    greet={false}
                />

                <div className="flex min-w-0 flex-col gap-0.5 pr-8">
                    <span className="text-[10px] font-semibold uppercase tracking-[0.18em] text-neutral-500">
                        {name}
                    </span>
                    <span className="truncate text-[15px] font-semibold text-neutral-100">
                        {notification.summary}
                    </span>
                    {notification.body && (
                        <span className="truncate text-[13px] text-neutral-400">
                            {notification.body}
                        </span>
                    )}
                </div>
            </div>

            <button
                type="button"
                aria-label="Dismiss notification"
                title="Dismiss notification"
                onClick={(event) => {
                    event.stopPropagation();
                    onClose();
                }}
                className="
                    absolute
                    right-3
                    top-3
                    flex
                    h-6
                    w-6
                    items-center
                    justify-center
                    rounded-md
                    text-neutral-500
                    transition-colors
                    duration-200
                    hover:bg-neutral-800
                    hover:text-neutral-100
                "
            >
                <X size={14} />
            </button>
        </div>
    );
}