import { useCallback, useEffect, useState, type ReactNode } from "react";
import { api } from "../services/api";
import { formatBytes } from "../services/time";
import { comboFromEvent, prettyShortcut } from "../services/shortcut";
import type { ImageUsage, LanguageSetting, Settings, ThemeSetting } from "../types";
import { useI18n, type MessageKey } from "../i18n";
import { BackIcon } from "./Icons";

interface Props {
  settings: Settings;
  onChange: (settings: Settings) => void;
  onBack: () => void;
}

const HISTORY_OPTIONS = [100, 250, 500, 1000, 2000];
const IMAGE_MB_OPTIONS = [50, 100, 200, 500, 1000];
const LANGUAGES: { value: LanguageSetting; label: MessageKey }[] = [
  { value: "system", label: "languageSystem" },
  { value: "tr", label: "languageTr" },
  { value: "en", label: "languageEn" },
];

const THEMES: { value: ThemeSetting; label: MessageKey }[] = [
  { value: "system", label: "themeSystem" },
  { value: "light", label: "themeLight" },
  { value: "dark", label: "themeDark" },
];

function ShortcutRecorder({
  value,
  onSave,
  onError,
}: {
  value: string;
  onSave: (combo: string) => Promise<void>;
  onError: (message: string) => void;
}) {
  const { t } = useI18n();
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (e.code === "Escape") {
        setRecording(false);
        return;
      }
      const combo = comboFromEvent(e);
      if (!combo) return;
      setRecording(false);
      onError("");
      onSave(combo).catch((err) => onError(String(err)));
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onSave, onError]);

  return (
    <button
      type="button"
      onClick={() => {
        onError("");
        setRecording((r) => !r);
      }}
      className={`min-w-[104px] rounded-lg px-3 py-1.5 text-[13px] transition-colors ${
        recording ? "bg-accent text-white" : "bg-fill text-ink hover:brightness-95"
      }`}
    >
      {recording ? t("shortcutRecording") : prettyShortcut(value)}
    </button>
  );
}

function Row({
  title,
  hint,
  error,
  children,
}: {
  title: string;
  hint?: string;
  error?: boolean;
  children: ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 px-4 py-3">
      <div className="min-w-0">
        <p className="text-[14px] text-ink">{title}</p>
        {hint && (
          <p
            className={`mt-0.5 text-xs leading-snug ${error ? "text-danger" : "text-muted"}`}
          >
            {hint}
          </p>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function Group({ children }: { children: ReactNode }) {
  return (
    <div className="divide-y divide-line overflow-hidden rounded-2xl bg-surface shadow-soft">
      {children}
    </div>
  );
}

function Switch({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (value: boolean) => void;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={`relative h-[26px] w-[44px] rounded-full transition-colors duration-200 ${
        checked ? "bg-accent" : "bg-fill"
      }`}
    >
      <span
        className={`absolute top-[2px] h-[22px] w-[22px] rounded-full bg-white shadow transition-all duration-200 ${
          checked ? "left-[20px]" : "left-[2px]"
        }`}
      />
    </button>
  );
}

function Select<T extends string | number>({
  value,
  options,
  format,
  onChange,
}: {
  value: T;
  options: T[];
  format: (v: T) => string;
  onChange: (v: T) => void;
}) {
  const list = options.includes(value) ? options : [...options, value].sort();
  return (
    <select
      value={value}
      onChange={(e) =>
        onChange((typeof value === "number" ? Number(e.target.value) : e.target.value) as T)
      }
      className="rounded-lg bg-fill px-2.5 py-1.5 text-[13px] text-ink outline-none"
    >
      {list.map((o) => (
        <option key={o} value={o}>
          {format(o)}
        </option>
      ))}
    </select>
  );
}

export function SettingsPanel({ settings, onChange, onBack }: Props) {
  const { t } = useI18n();
  const [usage, setUsage] = useState<ImageUsage>({ bytes: 0, count: 0 });

  useEffect(() => {
    api.imageUsage().then(setUsage).catch(() => {});
  }, [settings]);

  const [shortcutError, setShortcutError] = useState("");

  const update = useCallback(
    async (key: string, value: string | number | boolean) => {
      onChange(await api.setSetting(key, value));
    },
    [onChange],
  );
  const saveShortcut = useCallback(
    (combo: string) => update("shortcut", combo),
    [update],
  );

  return (
    <div className="absolute inset-0 z-20 flex flex-col bg-bg animate-slide-in">
      <header
        data-tauri-drag-region
        className="flex h-12 shrink-0 items-center gap-1 px-2"
      >
        <button
          type="button"
          onClick={onBack}
          className="flex items-center gap-0.5 rounded-lg py-1.5 pl-1 pr-2.5 text-[14px] text-accent transition-colors hover:bg-fill"
        >
          <BackIcon />
          {t("back")}
        </button>
        <h1 className="pointer-events-none flex-1 pr-14 text-center text-[15px] font-semibold text-ink">
          {t("settings")}
        </h1>
      </header>

      <div className="scroll-area flex-1 space-y-4 overflow-y-auto px-4 pb-5">
        <Group>
          <Row title={t("language")}>
            <Select<LanguageSetting>
              value={settings.language}
              options={LANGUAGES.map((l) => l.value)}
              format={(v) => t(LANGUAGES.find((l) => l.value === v)!.label)}
              onChange={(v) => update("language", v)}
            />
          </Row>
          <Row title={t("theme")}>
            <Select<ThemeSetting>
              value={settings.theme}
              options={THEMES.map((o) => o.value)}
              format={(v) => t(THEMES.find((o) => o.value === v)!.label)}
              onChange={(v) => update("theme", v)}
            />
          </Row>
          <Row
            title={t("shortcut")}
            hint={shortcutError || t("shortcutRecord")}
            error={Boolean(shortcutError)}
          >
            <ShortcutRecorder
              value={settings.shortcut}
              onSave={saveShortcut}
              onError={setShortcutError}
            />
          </Row>
          <Row title={t("autoPaste")} hint={t("autoPasteHint")}>
            <Switch
              checked={settings.autoPaste}
              label={t("autoPaste")}
              onChange={(v) => update("auto_paste", v)}
            />
          </Row>
          <Row title={t("autostart")} hint={t("autostartHint")}>
            <Switch
              checked={settings.autostart}
              label={t("autostart")}
              onChange={(v) => update("autostart", v)}
            />
          </Row>
        </Group>

        <Group>
          <Row title={t("historyLimit")} hint={t("historyLimitHint")}>
            <Select<number>
              value={settings.maxItems}
              options={HISTORY_OPTIONS}
              format={(v) => t("items", { n: v })}
              onChange={(v) => update("max_items", v)}
            />
          </Row>
          <Row title={t("saveImages")} hint={t("saveImagesHint")}>
            <Switch
              checked={settings.saveImages}
              label={t("saveImages")}
              onChange={(v) => update("save_images", v)}
            />
          </Row>
          <Row title={t("imageLimit")}>
            <Select<number>
              value={settings.maxImageMb}
              options={IMAGE_MB_OPTIONS}
              format={(v) => t("mb", { n: v })}
              onChange={(v) => update("max_image_mb", v)}
            />
          </Row>
          <Row
            title={t("imageUsage", {
              size: formatBytes(usage.bytes),
              n: usage.count,
            })}
          >
            <button
              type="button"
              disabled={usage.count === 0}
              onClick={async () => {
                await api.clearImages();
                setUsage(await api.imageUsage());
              }}
              className="rounded-lg bg-fill px-2.5 py-1.5 text-[13px] text-danger transition-colors hover:brightness-95 disabled:opacity-40"
            >
              {t("clearImages")}
            </button>
          </Row>
        </Group>

        <section className="px-1">
          <h2 className="text-[13px] font-semibold text-ink">{t("privacyTitle")}</h2>
          <p className="mt-1 text-xs leading-relaxed text-muted">{t("privacyBody")}</p>
          <p className="mt-3 text-xs text-muted">
            {t("shortcutHint", { key: prettyShortcut(settings.shortcut) })}
          </p>
        </section>
      </div>
    </div>
  );
}
