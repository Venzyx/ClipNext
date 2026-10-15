import { invoke } from "@tauri-apps/api/core";
import type { Clip, ImageUsage, Settings } from "../types";

export const api = {
  listClips: (query: string) =>
    invoke<Clip[]>("list_clips", { query: query.trim() || null }),
  copyClip: (id: number) => invoke<void>("copy_clip", { id }),
  togglePin: (id: number, pinned: boolean) =>
    invoke<void>("toggle_pin", { id, pinned }),
  deleteClip: (id: number) => invoke<void>("delete_clip", { id }),
  clearClips: (includePinned: boolean) =>
    invoke<void>("clear_clips", { includePinned }),
  clearImages: () => invoke<void>("clear_images"),
  getSettings: () => invoke<Settings>("get_settings"),
  setSetting: (key: string, value: string | number | boolean) =>
    invoke<Settings>("set_setting", { key, value: String(value) }),
  imageUsage: () => invoke<ImageUsage>("image_usage"),
  getThumbnail: (id: number) => invoke<string>("get_thumbnail", { id }),
  hideWindow: () => invoke<void>("hide_window"),
};
