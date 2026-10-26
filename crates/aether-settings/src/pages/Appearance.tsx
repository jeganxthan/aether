
import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Check,
  FolderOpen,
  ImagePlus,
  LoaderCircle,
  RefreshCw,
} from "lucide-react";

import ThemeSelector from "../components/ThemeSelector";
import WallpaperCard from "../components/wallpaper/WallpaperCard";
import {
  wallpaperService,
  type Wallpaper,
} from "../services/wallpaper";

export default function Appearance() {
  const [wallpapers, setWallpapers] = useState<Wallpaper[]>([]);
  const [current, setCurrent] = useState<string | null>(null);
  const [folder, setFolder] = useState("");
  const [loading, setLoading] = useState(true);
  const [applying, setApplying] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const loadWallpapers = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const [items, selected, directory] = await Promise.all([
        wallpaperService.list(),
        wallpaperService.current(),
        wallpaperService.folder(),
      ]);

      setWallpapers(items);
      setCurrent(selected);
      setFolder(directory);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadWallpapers();
  }, [loadWallpapers]);

  async function applyWallpaper(path: string) {
    setApplying(path);
    setError(null);
    setMessage(null);

    try {
      await wallpaperService.set(path);
      setCurrent(path);
      setMessage("Wallpaper applied successfully.");
    } catch (err) {
      setError(String(err));
    } finally {
      setApplying(null);
    }
  }

  async function browseWallpaper() {
    setError(null);

    try {
      const selected = await open({
        title: "Choose a wallpaper",
        multiple: false,
        directory: false,
        filters: [
          {
            name: "Images",
            extensions: [
              "jpg",
              "jpeg",
              "png",
              "webp",
              "bmp",
              "avif",
            ],
          },
        ],
      });

      if (typeof selected === "string") {
        await applyWallpaper(selected);
        await loadWallpapers();
      }
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="space-y-10">
      <header>
        <h1 className="text-[30px] font-semibold tracking-[-0.8px]">
          Appearance
        </h1>
        <p className="mt-2 text-sm text-neutral-400">
          Customize how Aether looks and feels.
        </p>
      </header>

      {/* Keep the existing theme selector as its own component. */}
      <section className="space-y-4">
        <div>
          <h2 className="text-base font-medium">Theme</h2>
          <p className="mt-1 text-sm text-neutral-400">
            Choose the visual style for your settings.
          </p>
        </div>

        <ThemeSelector />
      </section>

      <div className="h-px bg-white/[0.08]" />

      {/* Wallpaper section */}
      <section className="space-y-5">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div>
            <h2 className="text-base font-medium">Wallpaper</h2>
            <p className="mt-1 text-sm text-neutral-400">
              Choose a background for your desktop.
            </p>
          </div>

          <button
            type="button"
            onClick={() => void browseWallpaper()}
            disabled={applying !== null}
            className="inline-flex items-center gap-2 rounded-xl border border-white/10 bg-white/[0.04] px-4 py-2.5 text-sm transition hover:bg-white/[0.08] disabled:opacity-50"
          >
            <ImagePlus size={16} />
            Browse files
          </button>
        </div>

        {error && (
          <div
            role="alert"
            className="rounded-xl border border-red-400/20 bg-red-400/[0.06] p-3 text-sm text-red-300"
          >
            {error}
          </div>
        )}

        {message && (
          <div
            role="status"
            className="flex items-center gap-2 text-sm text-neutral-300"
          >
            <Check size={15} />
            {message}
          </div>
        )}

        <div className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-4">
          <div className="flex items-center justify-between gap-3">
            <div className="flex min-w-0 items-center gap-3">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-white/[0.05]">
                <FolderOpen size={18} className="text-neutral-300" />
              </div>

              <div className="min-w-0">
                <p className="text-sm font-medium">Wallpaper library</p>
                <p className="mt-1 break-all text-xs text-neutral-500">
                  {folder || "~/Pictures/Wallpapers"}
                </p>
              </div>
            </div>

            <button
              type="button"
              onClick={() => void loadWallpapers()}
              disabled={loading}
              title="Refresh wallpapers"
              className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg text-neutral-400 transition hover:bg-white/[0.06] hover:text-white disabled:opacity-50"
            >
              <RefreshCw
                size={15}
                className={loading ? "animate-spin" : ""}
              />
            </button>
          </div>
        </div>

        {loading ? (
          <div className="flex items-center justify-center gap-2 py-12 text-sm text-neutral-400">
            <LoaderCircle size={18} className="animate-spin" />
            Loading wallpapers…
          </div>
        ) : wallpapers.length === 0 ? (
          <div className="flex flex-col items-center justify-center rounded-2xl border border-dashed border-white/10 px-6 py-12 text-center">
            <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-2xl bg-white/[0.05]">
              <ImagePlus size={22} className="text-neutral-400" />
            </div>

            <p className="text-sm font-medium">No wallpapers in your library</p>
            <p className="mt-2 max-w-sm text-sm text-neutral-500">
              Browse for an image or add images to your Wallpapers folder.
            </p>

            <button
              type="button"
              onClick={() => void browseWallpaper()}
              className="mt-5 rounded-xl border border-white/10 px-4 py-2 text-sm transition hover:bg-white/[0.06]"
            >
              Choose an image
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
            {wallpapers.map((wallpaper) => (
              <WallpaperCard
                key={wallpaper.path}
                wallpaper={wallpaper}
                selected={current === wallpaper.path}
                disabled={applying !== null}
                onSelect={(path) => void applyWallpaper(path)}
              />
            ))}
          </div>
        )}

        {applying && (
          <div className="flex items-center gap-2 text-sm text-neutral-400">
            <LoaderCircle size={16} className="animate-spin" />
            Applying wallpaper…
          </div>
        )}
      </section>
    </div>
  );
}
