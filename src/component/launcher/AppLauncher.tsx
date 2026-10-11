import {
    useEffect,
    useRef,
    useState,
} from "react";

import { invoke } from "@tauri-apps/api/core";

interface AppInfo {
    id: string;
    name: string;
    exec: string;
    icon?: string | null;
    comment?: string | null;
    categories: string[];
}

interface LauncherResults {
    results: AppInfo[];
    autocomplete: string | null;
}

interface AppLauncherProps {
    /** Closes the launcher — hides the results and lets the island collapse. */
    onClose: () => void;
    /**
     * Report the number of results so the island can size itself to them.
     * Without this the island would keep a fixed height and show an empty
     * black slab whenever a search came back with nothing.
     */
    onResultCountChange?: (count: number) => void;
}

/*
 * Application icons are resolved lazily by the backend and turned into blob
 * URLs. The cache is module-level so re-opening the launcher (or re-rendering
 * a row) never re-reads the same icon from disk.
 */
const iconUrlCache = new Map<string, string | null>();

function AppIcon({
    icon,
    name,
}: {
    icon?: string | null;
    name: string;
}) {
    const [url, setUrl] = useState<string | null>(() =>
        icon ? iconUrlCache.get(icon) ?? null : null
    );

    useEffect(() => {
        if (!icon) {
            setUrl(null);
            return;
        }

        const cached = iconUrlCache.get(icon);

        if (cached !== undefined) {
            setUrl(cached);
            return;
        }

        let cancelled = false;

        invoke<[string, number[]]>("launcher_icon", { icon })
            .then(([, bytes]) => {
                const objectUrl = URL.createObjectURL(
                    new Blob([new Uint8Array(bytes)])
                );

                iconUrlCache.set(icon, objectUrl);

                if (!cancelled) setUrl(objectUrl);
            })
            .catch(() => {
                iconUrlCache.set(icon, null);

                if (!cancelled) setUrl(null);
            });

        return () => {
            cancelled = true;
        };
    }, [icon]);

    if (url) {
        return (
            <img
                src={url}
                alt=""
                draggable={false}
                className="
                    w-[34px]
                    h-[34px]
                    object-contain
                    select-none
                "
            />
        );
    }

    return (
        <span className="text-lg text-neutral-200">
            {name.charAt(0).toUpperCase()}
        </span>
    );
}

export default function AppLauncher({
    onClose,
    onResultCountChange,
}: AppLauncherProps) {
    const inputRef = useRef<HTMLInputElement>(null);
    const listRef = useRef<HTMLDivElement>(null);

    const [query, setQuery] = useState("");
    const [results, setResults] = useState<AppInfo[]>([]);

    const [selectedIndex, setSelectedIndex] =
        useState(0);

    /*
     * Focus immediately when the launcher opens. Under Wayland the surface
     * only gains keyboard focus a moment after the compositor is asked, so
     * we also re-focus whenever the window itself receives focus.
     */
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
        onResultCountChange?.(results.length);
    }, [results.length, onResultCountChange]);

    /*
     * Search applications. An empty query stays quiet — the launcher opens
     * as just an input bar and only grows once the user actually types.
     */
    useEffect(() => {
        if (query.trim() === "") {
            setResults([]);
            setSelectedIndex(0);
            return;
        }

        let cancelled = false;

        const search = async () => {
            try {
                const response =
                    await invoke<LauncherResults>(
                        "launcher_search",
                        {
                            query,
                        }
                    );

                if (cancelled) return;

                setResults(response.results);

                setSelectedIndex(0);
            } catch (error) {
                if (cancelled) return;

                console.error(
                    "Launcher search failed:",
                    error
                );

                setResults([]);
            }
        };

        search();

        return () => {
            cancelled = true;
        };
    }, [query]);

    /*
     * Keep the highlighted row inside the visible part of the dropdown as
     * the user walks the list with the arrow keys — otherwise the selection
     * moves past the last visible row and the highlight disappears.
     */
    useEffect(() => {
        const node = listRef.current?.children[selectedIndex];

        if (node instanceof HTMLElement) {
            node.scrollIntoView({ block: "nearest" });
        }
    }, [selectedIndex]);

    /*
     * A fresh search starts at the top of the list.
     */
    useEffect(() => {
        if (listRef.current) {
            listRef.current.scrollTop = 0;
        }
    }, [results]);

    /*
     * Launch selected application.
     */
    const launchSelected = async () => {
        const app = results[selectedIndex];

        if (!app) return;

        try {
            await invoke(
                "launcher_launch",
                {
                    app,
                }
            );

            onClose();
        } catch (error) {
            console.error(
                "Failed to launch application:",
                error
            );
        }
    };

    /*
     * Keyboard navigation.
     */
    const handleKeyDown = (
        event: React.KeyboardEvent<HTMLInputElement>
    ) => {
        switch (event.key) {
            case "ArrowDown":
                event.preventDefault();

                setSelectedIndex((index) =>
                    Math.min(
                        index + 1,
                        results.length - 1
                    )
                );

                break;

            case "ArrowUp":
                event.preventDefault();

                setSelectedIndex((index) =>
                    Math.max(index - 1, 0)
                );

                break;

            case "Enter":
                event.preventDefault();

                launchSelected();

                break;

            case "Escape":
                event.preventDefault();

                onClose();

                break;
        }
    };

    return (
        <div
            className="
                pointer-events-none

                absolute
                inset-0

                flex
                flex-col
            "
        >
            {/*
                Search row. It matches the island's launcher header height so
                the input sits dead-centre beside Mochi; the left padding
                clears Mochi's own box, which the island positions separately.
            */}
            <div
                className="
                    h-[64px]
                    shrink-0

                    flex
                    items-center

                    pl-[101px]
                    pr-6
                "
            >
                <input
                    ref={inputRef}
                    value={query}
                    onChange={(event) =>
                        setQuery(
                            event.target.value
                        )
                    }
                    onKeyDown={handleKeyDown}
                    placeholder="Search applications..."
                    className="
                        pointer-events-auto

                        w-full

                        bg-transparent
                        outline-none

                        text-[18px]
                        text-neutral-100

                        placeholder:text-neutral-600
                    "
                    autoComplete="off"
                    spellCheck={false}
                />
            </div>

            {/* Results */}

            {results.length > 0 && (
                <div
                    ref={listRef}
                    className="
                        no-scrollbar

                        pointer-events-auto

                        min-h-0
                        flex-1

                        overflow-y-auto

                        border-t
                        border-neutral-800

                        px-2
                        py-2
                    "
                >
                    {results.map(
                        (app, index) => (
                            <button
                                key={app.id}
                                type="button"
                                onClick={() => {
                                    setSelectedIndex(
                                        index
                                    );

                                    launchSelected();
                                }}
                                className={`
                                        w-full

                                        h-[56px]

                                        px-3

                                        flex
                                        items-center

                                        text-left

                                        rounded-[12px]

                                        transition-colors

                                        ${selectedIndex ===
                                            index
                                            ? "bg-neutral-900"
                                            : "bg-transparent"
                                        }

                                        hover:bg-neutral-900
                                    `}
                            >
                                <div
                                    className="
                                            w-[36px]
                                            h-[36px]

                                            flex
                                            items-center
                                            justify-center

                                            shrink-0
                                        "
                                >
                                    <AppIcon
                                        icon={app.icon}
                                        name={app.name}
                                    />
                                </div>

                                <div className="ml-3 min-w-0">
                                    <div
                                        className="
                                                text-[14px]
                                                text-neutral-100
                                            "
                                    >
                                        {app.name}
                                    </div>

                                    {app.comment && (
                                        <div
                                            className="
                                                    mt-0.5

                                                    truncate

                                                    text-[11px]
                                                    text-neutral-500
                                                "
                                        >
                                            {
                                                app.comment
                                            }
                                        </div>
                                    )}
                                </div>
                            </button>
                        )
                    )}
                </div>
            )}
        </div>
    );
}
