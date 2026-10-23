// The UI's words, in Project Fluent: one file per language,
// `locales/<lang>/feedback.ftl`, in which English decides which messages
// exist (docs/translations.md). The page speaks the browser's language when
// it is one of these, else English, and a message a language lacks reads in
// English. Call sites name a message as `section.name`; the file spells the
// same id `section-name`, because a Fluent id takes no dot.

import { FluentBundle, FluentResource } from "@fluent/bundle";
import de from "../locales/de/feedback.ftl?raw";
import en from "../locales/en/feedback.ftl?raw";
import type { Key } from "./i18n-keys";

export type { Key } from "./i18n-keys";

/**
 * Every language: the locale its numbers, dates and plural forms follow, and
 * its file. A language is added here and nowhere else in the code.
 */
const LANGS = {
  en: { locale: "en-GB", source: en },
  de: { locale: "de-DE", source: de },
} as const;

export type Lang = keyof typeof LANGS;

/** Every language, English first. */
export const languages = Object.keys(LANGS) as Lang[];

/** The locale `lang` formats numbers and dates in. */
export function localeOf(lang: Lang): string {
  return LANGS[lang].locale;
}

const bundles = new Map<Lang, FluentBundle>();

/** `lang`'s messages, parsed on first use. */
function bundle(lang: Lang): FluentBundle {
  let found = bundles.get(lang);
  if (found === undefined) {
    // No isolation marks around arguments: every language here is written
    // left to right, and the marks would reach the DOM as invisible text.
    found = new FluentBundle(LANGS[lang].locale, { useIsolating: false });
    found.addResource(new FluentResource(LANGS[lang].source));
    bundles.set(lang, found);
  }
  return found;
}

const fluentId = (key: string) => key.replaceAll(".", "-");

/**
 * `key` in `lang`, its `{ $name }` arguments filled from `values`; in English
 * where `lang` lacks it, and the key itself where English does too. A number
 * is written as `lang` writes it (1,234 or 1.234) and picks the plural form,
 * so a count is passed as a number and an identifier (an issue number, a
 * version) as a string. A missing argument shows as `{$name}`.
 */
export function t(lang: Lang, key: Key, values: Record<string, string | number> = {}): string {
  const id = fluentId(key);
  for (const each of lang === "en" ? [bundle("en")] : [bundle(lang), bundle("en")]) {
    const pattern = each.getMessage(id)?.value;
    if (pattern !== undefined && pattern !== null) return each.formatPattern(pattern, values, []);
  }
  return key;
}

/** `value` named by the message `prefix.value`, or `value` itself if there is none. */
function label(lang: Lang, prefix: string, value: string): string {
  const key = `${prefix}.${value}`;
  return bundle("en").hasMessage(fluentId(key)) ? t(lang, key as Key) : value;
}

/** A status as `lang` names it. */
export function statusLabel(lang: Lang, status: string): string {
  return label(lang, "status", status);
}

/** A priority as `lang` names it. */
export function priorityLabel(lang: Lang, priority: string): string {
  return label(lang, "prio", priority);
}

/** What the list is sorted by, as `lang` names it. */
export function sortLabel(lang: Lang, sort: string): string {
  return label(lang, "sort", sort);
}

/** A kind as `lang` names it. */
export function kindLabel(lang: Lang, kind: string): string {
  return label(lang, "kind", kind);
}

/** The browser's language, if it is one of ours; English otherwise. */
export function pickLang(tags: readonly string[]): Lang {
  for (const tag of tags) {
    const lower = tag.toLowerCase();
    const base = lower.split("-")[0];
    const found = languages.find((code) => code.toLowerCase() === lower || code.toLowerCase() === base);
    if (found !== undefined) return found;
  }
  return "en";
}

export function browserLang(): Lang {
  return typeof navigator === "undefined" ? "en" : pickLang(navigator.languages);
}

/** A number as `lang` writes it: 1,234 or 1.234. */
export function formatCount(lang: Lang, n: number): string {
  return new Intl.NumberFormat(localeOf(lang)).format(n);
}

/** An ISO time as `lang` reads it, in the browser's time zone, to the minute. */
export function formatWhen(lang: Lang, iso: string): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  return new Intl.DateTimeFormat(localeOf(lang), {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(at);
}

/** How long ago (or until) `iso` is, from `now`: "5 minutes ago". */
export function formatAgo(lang: Lang, iso: string, now: number = Date.now()): string {
  const at = new Date(iso).getTime();
  if (Number.isNaN(at)) return iso;
  const seconds = Math.round((at - now) / 1000);
  const rtf = new Intl.RelativeTimeFormat(localeOf(lang), { numeric: "auto" });
  const abs = Math.abs(seconds);
  if (abs < 45) return rtf.format(0, "second");
  if (abs < 3600) return rtf.format(Math.round(seconds / 60), "minute");
  if (abs < 86_400) return rtf.format(Math.round(seconds / 3600), "hour");
  if (abs < 30 * 86_400) return rtf.format(Math.round(seconds / 86_400), "day");
  if (abs < 365 * 86_400) return rtf.format(Math.round(seconds / (30 * 86_400)), "month");
  return rtf.format(Math.round(seconds / (365 * 86_400)), "year");
}

/** A day as `lang` writes it short: 9 Oct or 9. Okt. */
export function formatDay(lang: Lang, day: string): string {
  const at = new Date(`${day}T00:00:00Z`);
  if (Number.isNaN(at.getTime())) return day;
  return new Intl.DateTimeFormat(localeOf(lang), { day: "numeric", month: "short", timeZone: "UTC" }).format(at);
}

/** A span of seconds as the two largest units: "3 d 4 h", "12 min". */
export function formatDuration(lang: Lang, secs: number): string {
  const s = Math.max(0, Math.round(secs));
  const d = Math.floor(s / 86_400);
  const h = Math.floor((s % 86_400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  const unit = (n: number, u: "day" | "hour" | "minute" | "second") =>
    new Intl.NumberFormat(localeOf(lang), { style: "unit", unit: u, unitDisplay: "narrow" }).format(n);
  if (d > 0) return h > 0 ? `${unit(d, "day")} ${unit(h, "hour")}` : unit(d, "day");
  if (h > 0) return m > 0 ? `${unit(h, "hour")} ${unit(m, "minute")}` : unit(h, "hour");
  if (m > 0) return unit(m, "minute");
  return unit(s, "second");
}

/** Bytes as kB, MB, GB, one decimal where it helps. */
export function formatBytes(lang: Lang, bytes: number): string {
  const units = ["B", "kB", "MB", "GB", "TB"];
  let value = bytes;
  let i = 0;
  while (value >= 1000 && i < units.length - 1) {
    value /= 1000;
    i += 1;
  }
  const digits = i === 0 || value >= 100 ? 0 : 1;
  return `${new Intl.NumberFormat(localeOf(lang), { maximumFractionDigits: digits }).format(value)} ${units[i] ?? "B"}`;
}
