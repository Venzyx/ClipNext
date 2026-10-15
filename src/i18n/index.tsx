import { createContext, useContext, useMemo, type ReactNode } from "react";
import { en, type Messages } from "./en";
import { tr } from "./tr";
import type { LanguageSetting } from "../types";

export type Lang = "tr" | "en";
export type MessageKey = keyof Messages;
type Vars = Record<string, string | number>;

const dictionaries: Record<Lang, Messages> = { en, tr };

export function resolveLanguage(setting: LanguageSetting): Lang {
  if (setting === "tr" || setting === "en") return setting;
  return navigator.language.toLowerCase().startsWith("tr") ? "tr" : "en";
}

interface I18n {
  lang: Lang;
  t: (key: MessageKey, vars?: Vars) => string;
}

const I18nContext = createContext<I18n>({
  lang: "en",
  t: (key) => en[key],
});

export function I18nProvider({
  setting,
  children,
}: {
  setting: LanguageSetting;
  children: ReactNode;
}) {
  const value = useMemo<I18n>(() => {
    const lang = resolveLanguage(setting);
    const dict = dictionaries[lang];
    return {
      lang,
      t: (key, vars) =>
        Object.entries(vars ?? {}).reduce(
          (text, [k, v]) => text.split(`{${k}}`).join(String(v)),
          dict[key],
        ),
    };
  }, [setting]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18n {
  return useContext(I18nContext);
}
