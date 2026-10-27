// Minimal, dependency-free i18n. To add a language: create `<locale>.ts`
// typed as `Dictionary`, then register it in `DICTIONARIES` and `LOCALES`.
import en from "./en";
import ptBR from "./pt-BR";

export type Dictionary = typeof en;
export type Locale = "en" | "pt-BR";

const DICTIONARIES: Record<Locale, Dictionary> = { en, "pt-BR": ptBR };

export const LOCALES: { id: Locale; label: string }[] = [
  { id: "en", label: "English" },
  { id: "pt-BR", label: "Português (Brasil)" },
];

const STORAGE_KEY = "stackquake.locale";

function isLocale(v: unknown): v is Locale {
  return typeof v === "string" && v in DICTIONARIES;
}

function detect(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (isLocale(saved)) return saved;
  } catch {
    // Storage unavailable: fall back to the system language.
  }
  return navigator.language?.toLowerCase().startsWith("pt") ? "pt-BR" : "en";
}

export const i18n = $state<{ locale: Locale }>({ locale: detect() });
document.documentElement.lang = i18n.locale;

export function setLocale(locale: Locale) {
  i18n.locale = locale;
  document.documentElement.lang = locale;
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    // Not persisted; the choice still applies to this session.
  }
}

function lookup(dict: unknown, key: string): string | undefined {
  let node: unknown = dict;
  for (const part of key.split(".")) {
    if (node === null || typeof node !== "object") return undefined;
    node = (node as Record<string, unknown>)[part];
  }
  return typeof node === "string" ? node : undefined;
}

export type Params = Record<string, string | number | null | undefined>;

/** Translates `key` (dotted path) with `{name}` placeholders. Falls back to English, then to the key. */
export function t(key: string, params?: Params): string {
  const text = lookup(DICTIONARIES[i18n.locale], key) ?? lookup(en, key) ?? key;
  return text.replace(/\{(\w+)\}/g, (_, name: string) => {
    const v = params?.[name];
    return v === null || v === undefined ? "—" : String(v);
  });
}

/** Like `t`, but returns `undefined` when the key does not exist. */
export function tMaybe(key: string): string | undefined {
  return lookup(DICTIONARIES[i18n.locale], key) ?? lookup(en, key);
}

/** BCP 47 tag for `Intl` formatting. */
export function intlLocale(): string {
  return i18n.locale;
}
