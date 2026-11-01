import { useCallback, useEffect, useRef, useState } from "react";
import { ClipList } from "./components/ClipList";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { CloseIcon, GearIcon, SearchIcon } from "./components/Icons";
import { SettingsPanel } from "./components/SettingsPanel";
import { useClips, useTauriEvent } from "./hooks/useClips";
import { I18nProvider, useI18n } from "./i18n";
import { api } from "./services/api";
import type { Clip, Settings } from "./types";

const DEFAULT_SETTINGS: Settings = {
  maxItems: 500,
  maxImageMb: 200,
  saveImages: true,
  autoPaste: true,
  language: "system",
  theme: "system",
  shortcut: "Control+Shift+KeyV",
  autostart: false,
};

function useDebounced<T>(value: T, ms: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), ms);
    return () => clearTimeout(id);
  }, [value, ms]);
  return debounced;
}

function Main({
  settings,
  onSettingsChange,
}: {
  settings: Settings;
  onSettingsChange: (s: Settings) => void;
}) {
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const debouncedQuery = useDebounced(query, 120);
  const { clips } = useClips(debouncedQuery);
  const [selected, setSelected] = useState(0);
  // Ids of the images picked for a multi-paste, in the order they were picked.
  const [multi, setMulti] = useState<number[]>([]);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [clearInfo, setClearInfo] = useState<{ count: number; pinned: number } | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => setSelected(0), [debouncedQuery]);
  useEffect(() => {
    setSelected((s) => Math.min(s, Math.max(clips.length - 1, 0)));
  }, [clips.length]);

  // Forget selected images that were deleted or filtered out of the history.
  useEffect(() => {
    setMulti((ids) => {
      const kept = ids.filter((id) => clips.some((c) => c.id === id));
      return kept.length === ids.length ? ids : kept;
    });
  }, [clips]);

  const onShown = useCallback(() => {
    setQuery("");
    setMulti([]);
    setSelected(0);
    setSettingsOpen(false);
    setClearInfo(null);
    searchRef.current?.focus();
  }, []);
  useTauriEvent("window-shown", onShown);
  useTauriEvent("open-settings", useCallback(() => setSettingsOpen(true), []));

  const activate = useCallback((clip: Clip) => {
    void api.copyClip(clip.id);
  }, []);
  const toggleMulti = useCallback((clip: Clip) => {
    if (clip.kind !== "image") return;
    setMulti((ids) =>
      ids.includes(clip.id) ? ids.filter((id) => id !== clip.id) : [...ids, clip.id],
    );
  }, []);
  const pasteMulti = useCallback(() => {
    if (multi.length > 0) void api.pasteClips(multi);
  }, [multi]);
  const togglePin = useCallback((clip: Clip) => {
    void api.togglePin(clip.id, !clip.pinned);
  }, []);
  const remove = useCallback((clip: Clip) => {
    void api.deleteClip(clip.id);
  }, []);

  const askClear = async () => {
    const all = await api.listClips("");
    setClearInfo({ count: all.length, pinned: all.filter((c) => c.pinned).length });
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === ",") {
        e.preventDefault();
        setSettingsOpen((open) => !open);
        return;
      }
      if (clearInfo) {
        if (e.key === "Escape") setClearInfo(null);
        return;
      }
      if (settingsOpen) {
        if (e.key === "Escape") setSettingsOpen(false);
        return;
      }
      const current = clips[selected];
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelected((s) => Math.min(s + 1, clips.length - 1));
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelected((s) => Math.max(s - 1, 0));
          break;
        case "Enter":
          if (multi.length > 0) {
            e.preventDefault();
            pasteMulti();
          } else if (current) {
            e.preventDefault();
            activate(current);
          }
          break;
        case " ":
          if ((e.ctrlKey || e.metaKey) && current) {
            e.preventDefault();
            toggleMulti(current);
          }
          break;
        case "Escape":
          if (multi.length > 0) setMulti([]);
          else if (query) setQuery("");
          else void api.hideWindow();
          break;
        case "Delete":
          if (!query && current) remove(current);
          break;
        case "p":
        case "P":
          if ((e.ctrlKey || e.metaKey) && current) {
            e.preventDefault();
            togglePin(current);
          }
          break;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [
    clips,
    selected,
    query,
    multi,
    settingsOpen,
    clearInfo,
    activate,
    remove,
    togglePin,
    toggleMulti,
    pasteMulti,
  ]);

  return (
    <div className="relative flex h-full flex-col overflow-hidden border border-line bg-bg text-ink">
      <header
        data-tauri-drag-region
        className="flex h-11 shrink-0 items-center justify-between pl-4 pr-2"
      >
        <span className="pointer-events-none text-[13px] font-semibold tracking-wide text-muted">
          {t("appName")}
        </span>
        <div className="flex items-center gap-0.5">
          <button
            type="button"
            title={t("settings")}
            aria-label={t("settings")}
            onClick={() => setSettingsOpen(true)}
            className="rounded-lg p-1.5 text-muted transition-colors hover:bg-fill hover:text-ink"
          >
            <GearIcon />
          </button>
          <button
            type="button"
            title={t("close")}
            aria-label={t("close")}
            onClick={() => void api.hideWindow()}
            className="rounded-lg p-1.5 text-muted transition-colors hover:bg-fill hover:text-ink"
          >
            <CloseIcon />
          </button>
        </div>
      </header>

      <div className="shrink-0 px-3 pb-2">
        <label className="flex items-center gap-2 rounded-xl bg-fill px-3 py-2 text-muted focus-within:ring-2 focus-within:ring-accent/40">
          <SearchIcon width={16} height={16} />
          <input
            ref={searchRef}
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("searchPlaceholder")}
            spellCheck={false}
            className="w-full bg-transparent text-[14px] text-ink outline-none placeholder:text-muted"
          />
        </label>
      </div>

      <ClipList
        clips={clips}
        selectedIndex={selected}
        searching={debouncedQuery.trim().length > 0}
        multi={multi}
        onSelect={setSelected}
        onActivate={activate}
        onToggleMulti={toggleMulti}
        onTogglePin={togglePin}
        onDelete={remove}
      />

      {multi.length > 0 ? (
        <footer className="flex shrink-0 items-center justify-between gap-2 border-t border-line bg-surface px-3 py-2.5 animate-fade-in">
          <span className="pl-1 text-[13px] font-medium text-ink">
            {t("multiSelected", { n: multi.length })}
          </span>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setMulti([])}
              className="rounded-lg bg-fill px-3 py-1.5 text-[13px] text-ink transition-colors hover:brightness-95"
            >
              {t("cancel")}
            </button>
            <button
              type="button"
              onClick={pasteMulti}
              className="rounded-lg bg-accent px-3 py-1.5 text-[13px] font-medium text-white transition-colors hover:brightness-95"
            >
              {t("pasteSelected", { n: multi.length })}
            </button>
          </div>
        </footer>
      ) : (
        <footer className="flex shrink-0 items-center justify-between border-t border-line px-4 py-2.5 text-xs text-muted">
          <span>{t("itemCount", { n: clips.length })}</span>
          <button
            type="button"
            onClick={() => void askClear()}
            className="rounded-md px-1.5 py-0.5 transition-colors hover:text-danger"
          >
            {t("clearAll")}
          </button>
        </footer>
      )}

      {settingsOpen && (
        <SettingsPanel
          settings={settings}
          onChange={onSettingsChange}
          onBack={() => {
            setSettingsOpen(false);
            searchRef.current?.focus();
          }}
        />
      )}
      {clearInfo && (
        <ConfirmDialog
          count={clearInfo.count}
          pinnedCount={clearInfo.pinned}
          onCancel={() => setClearInfo(null)}
          onConfirm={(includePinned) => {
            setClearInfo(null);
            void api.clearClips(includePinned);
          }}
        />
      )}
    </div>
  );
}

function App() {
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);

  useEffect(() => {
    api.getSettings().then(setSettings).catch((e) => console.error(e));
  }, []);

  useEffect(() => {
    const root = document.documentElement;
    if (settings.theme === "system") delete root.dataset.theme;
    else root.dataset.theme = settings.theme;
  }, [settings.theme]);

  return (
    <I18nProvider setting={settings.language}>
      <Main settings={settings} onSettingsChange={setSettings} />
    </I18nProvider>
  );
}

export default App;
