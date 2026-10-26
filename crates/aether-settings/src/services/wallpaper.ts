import { invoke } from "@tauri-apps/api/core";

export interface Wallpaper {
  path: string;
  name: string;
}

export const wallpaperService = {
  list: () => invoke<Wallpaper[]>("wallpaper_list"),

  current: () => invoke<string | null>("wallpaper_current"),

  folder: () => invoke<string>("wallpaper_folder"),

  set: (path: string) => invoke<void>("wallpaper_set", { path }),

  restore: () => invoke<void>("wallpaper_restore"),
};