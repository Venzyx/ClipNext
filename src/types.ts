export type ClipKind = "text" | "link" | "image";

export interface Clip {
  id: number;
  kind: ClipKind;
  text: string | null;
  imagePath: string | null;
  thumbPath: string | null;
  pinned: boolean;
  createdAt: number;
  lastUsedAt: number;
}

export type LanguageSetting = "system" | "tr" | "en";
export type ThemeSetting = "system" | "light" | "dark";

export interface Settings {
  maxItems: number;
  maxImageMb: number;
  saveImages: boolean;
  autoPaste: boolean;
  language: LanguageSetting;
  theme: ThemeSetting;
  shortcut: string;
  autostart: boolean;
}

export interface ImageUsage {
  bytes: number;
  count: number;
}
