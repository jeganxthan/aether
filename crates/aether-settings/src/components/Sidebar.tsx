
import { useState } from "react";
import {
    Menu,
    Search,
    Settings2,
    Palette,
    Monitor,
    Wifi,
    Bluetooth,
    Volume2,
    Zap,
    Bell,
    Keyboard,
    Info,
    X,
} from "lucide-react";

import aetherLogo from "../assets/aether.png";

export type SettingsPage =
    | "General"
    | "Appearance"
    | "Display"
    | "Network"
    | "Bluetooth"
    | "Sound"
    | "Power"
    | "Notifications"
    | "Keyboard"
    | "About";

interface SidebarProps {
    activePage: SettingsPage;
    onNavigate: (page: SettingsPage) => void;
}

const navigation = [
    { label: "General", icon: Settings2 },
    { label: "Appearance", icon: Palette },
    { label: "Display", icon: Monitor },
    { label: "Network", icon: Wifi },
    { label: "Bluetooth", icon: Bluetooth },
    { label: "Sound", icon: Volume2 },
    { label: "Power", icon: Zap },
    { label: "Notifications", icon: Bell },
    { label: "Keyboard", icon: Keyboard },
    { label: "About", icon: Info },
] as const;

export default function Sidebar({
    activePage,
    onNavigate,
}: SidebarProps) {
    const [search, setSearch] = useState("");
    const [collapsed, setCollapsed] = useState(false);

    const filteredNavigation = navigation.filter(({ label }) =>
        label.toLowerCase().includes(search.trim().toLowerCase()),
    );

    return (
        <aside
            className={`relative isolate flex h-full shrink-0 flex-col overflow-hidden bg-transparent py-5 text-neutral-100 transition-[width] duration-200 ease-out ${collapsed ? "w-[76px]" : "w-[350px]"
                }`}
        >
            {/* Brand and collapse button */}

            {/* Brand and collapse button */}
            <div
                className={`mb-7 flex min-h-12 shrink-0 items-center ${collapsed
                        ? "justify-center px-2"
                        : "justify-between gap-3 px-[18px]"
                    }`}
            >
                {!collapsed && (
                    <div className="flex min-w-0 items-center gap-3">
                        <img
                            src={aetherLogo}
                            alt="Aether"
                            draggable={false}
                            className="h-[38px] w-[38px] shrink-0 object-contain"
                        />

                        <span className="text-[25px] font-semibold tracking-[-0.8px]">
                            Aether
                        </span>
                    </div>
                )}

                <button
                    type="button"
                    onClick={() => {
                        setCollapsed((value) => !value);
                        setSearch("");
                    }}
                    title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
                    aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
                    aria-expanded={!collapsed}
                    className="grid h-9 w-9 shrink-0 place-items-center rounded-xl border border-white/10 text-neutral-300 transition-colors hover:bg-white/10 hover:text-white"
                >
                    <Menu size={19} strokeWidth={1.8} />
                </button>
            </div>


            {/* Search — hidden when collapsed */}
            {!collapsed && (
                <label className="mx-[18px] mb-6 flex h-[46px] shrink-0 items-center gap-[11px] rounded-[13px] border border-white/[0.14] bg-white/[0.035] px-[14px] text-neutral-400 transition-colors focus-within:border-violet-400/70 focus-within:bg-white/[0.055]">
                    <Search size={19} strokeWidth={1.8} />

                    <input
                        type="search"
                        placeholder="Search settings..."
                        aria-label="Search settings"
                        value={search}
                        onChange={(event) => setSearch(event.target.value)}
                        className="min-w-0 flex-1 bg-transparent text-[13px] text-neutral-100 outline-none placeholder:text-neutral-500"
                    />

                    {search && (
                        <button
                            type="button"
                            aria-label="Clear search"
                            onClick={() => setSearch("")}
                            className="rounded p-0.5 hover:bg-white/10 hover:text-white"
                        >
                            <X size={16} />
                        </button>
                    )}
                </label>
            )}

            {/* Navigation — scrollbar hidden, scrolling preserved */}
            <nav
                aria-label="Settings navigation"
                className={`flex min-h-0 flex-1 flex-col gap-[5px] overflow-y-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden ${collapsed ? "items-center px-2" : "px-[18px]"
                    }`}
            >
                {filteredNavigation.map(({ label, icon: Icon }) => {
                    const selected = activePage === label;

                    return (
                        <button
                            key={label}
                            type="button"
                            aria-current={selected ? "page" : undefined}
                            aria-label={label}
                            title={collapsed ? label : undefined}
                            onClick={() => onNavigate(label)}
                            className={`relative flex min-h-[49px] w-full shrink-0 items-center rounded-[11px] text-sm transition-colors duration-150 ${collapsed
                                    ? "justify-center px-0"
                                    : "gap-[17px] px-5"
                                } ${selected
                                    ? "bg-white/[0.17] text-white"
                                    : "text-neutral-200 hover:bg-white/[0.075]"
                                }`}
                        >
                            {selected && (
                                <span className="absolute bottom-[7px] left-0 top-[7px] w-[3px] rounded-r shadow-[0_0_12px_rgba(155,122,255,0.24)]" />
                            )}

                            <Icon
                                size={23}
                                strokeWidth={1.7}
                                className="shrink-0"
                            />

                            {!collapsed && (
                                <span className="truncate">{label}</span>
                            )}
                        </button>
                    );
                })}

                {!collapsed && filteredNavigation.length === 0 && (
                    <p className="px-3 py-3 text-[13px] text-neutral-400">
                        No settings found
                    </p>
                )}
            </nav>

            {/* Footer */}
            <div
                className={`mt-3 flex shrink-0 items-center border-t border-white/[0.09] pt-[18px] text-[11px] text-neutral-400 ${collapsed
                        ? "justify-center px-2"
                        : "gap-[9px] px-[28px]"
                    }`}
            >
                <span className="h-[7px] w-[7px] shrink-0 rounded-full bg-[var(--accent)]  shadow-[0_0_9px_rgba(155,122,255,0.45)]" />

                {!collapsed && (
                    <>
                        <span>Aether Desktop</span>
                        <span className="ml-auto tabular-nums text-neutral-600">
                            0.1.0
                        </span>
                    </>
                )}
            </div>
        </aside>
    );
}
