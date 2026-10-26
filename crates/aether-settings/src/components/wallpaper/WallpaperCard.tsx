import { Check } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Wallpaper } from "../../services/wallpaper";

interface WallpaperCardProps {
  wallpaper: Wallpaper;
  selected: boolean;
  onSelect: (path: string) => void;
  disabled?: boolean;
}

export default function WallpaperCard({
  wallpaper,
  selected,
  onSelect,
  disabled = false,
}: WallpaperCardProps) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={() => onSelect(wallpaper.path)}
      aria-pressed={selected}
      title={wallpaper.name}
      className={[
        "group overflow-hidden rounded-xl border text-left transition",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent)]",
        selected
          ? "border-[var(--accent)] ring-1 ring-[var(--accent)]"
          : "border-white/10 hover:border-white/25",
        disabled ? "cursor-wait opacity-60" : "",
      ].join(" ")}
    >
      <div className="relative aspect-video overflow-hidden bg-white/[0.04]">
        <img
          src={convertFileSrc(wallpaper.path)}
          alt={wallpaper.name}
          loading="lazy"
          className="h-full w-full object-cover transition duration-300 group-hover:scale-[1.03]"
        />

        {selected && (
          <span className="absolute right-2 top-2 flex h-6 w-6 items-center justify-center rounded-full bg-black/75 text-white">
            <Check size={14} />
          </span>
        )}
      </div>

      <div className="flex items-center gap-2 px-3 py-2.5">
        <span className="min-w-0 flex-1 truncate text-sm text-white/80">
          {wallpaper.name}
        </span>

        {selected && (
          <span className="shrink-0 text-xs text-white/50">Active</span>
        )}
      </div>
    </button>
  );
}