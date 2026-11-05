/*
 * Every setting of this page, kept in localStorage. Each read and write is
 * guarded: in a private window, or with storage blocked, the page works the
 * same and only forgets. Nothing here ever reaches the service.
 */

const KEY = "baylee-pulse.settings.v1";

export const ACCENTS = ["mint", "blau", "violett", "koralle", "gold", "rosa"];
export const RANGES = [60, 300, 900, 3600, 21600, 86400];

export const DEFAULT_ALERTS = {
  diskWarn: 85,
  diskCrit: 95,
  diskFreeGiB: 20,
  ramWarn: 90,
  ramCrit: 97,
  swapWarn: 50,
  swapCrit: 85,
  cpuTempWarn: 85,
  cpuTempCrit: 95,
  gpuTempWarn: 84,
  gpuTempCrit: 92,
  diskTempWarn: 70,
  diskTempCrit: 80,
  gpuIdleMinutes: 5,
  notify: false,
  sound: false,
};

export const DEFAULT_COLUMNS = ["name", "state", "cpu", "ram", "swap", "gpu", "vram", "read", "write", "threads", "sockets", "runtime"];

function defaults() {
  return {
    theme: "system",
    accent: "mint",
    density: "normal",
    rateUnit: "iec",
    cpuScale: "core",
    range: 300,
    showVirtual: false,
    alerts: { ...DEFAULT_ALERTS },
    processes: { sort: "cpu", descending: true, filter: "all", group: false, columns: [...DEFAULT_COLUMNS] },
    layout: { order: [], hidden: [], sizes: {} },
  };
}

const oneOf = (value, allowed, fallback) => (allowed.includes(value) ? value : fallback);
const clamp = (value, low, high, fallback) => (Number.isFinite(value) ? Math.min(high, Math.max(low, value)) : fallback);
const words = value => (Array.isArray(value) ? value.filter(item => typeof item === "string").slice(0, 200) : []);

function cleanAlerts(raw) {
  const clean = { ...DEFAULT_ALERTS };
  for (const [key, fallback] of Object.entries(DEFAULT_ALERTS)) {
    const value = raw?.[key];
    clean[key] = typeof fallback === "boolean" ? value === true : clamp(value, 0, 10000, fallback);
  }
  return clean;
}

function cleanLayout(raw) {
  const sizes = {};
  for (const [id, size] of Object.entries(raw?.sizes ?? {})) {
    if (typeof id !== "string" || typeof size !== "object" || size == null) continue;
    sizes[id] = { w: clamp(Math.round(size.w), 1, 12, 4), h: clamp(Math.round(size.h), 1, 14, 3) };
  }
  return { order: words(raw?.order), hidden: words(raw?.hidden), sizes };
}

/** Keeps what is known and valid from any stored or imported object. */
export function clean(raw) {
  const base = defaults();
  const processes = raw?.processes;
  return {
    theme: oneOf(raw?.theme, ["system", "light", "dark"], base.theme),
    accent: oneOf(raw?.accent, ACCENTS, base.accent),
    density: oneOf(raw?.density, ["normal", "compact"], base.density),
    rateUnit: oneOf(raw?.rateUnit, ["iec", "si", "bits"], base.rateUnit),
    cpuScale: oneOf(raw?.cpuScale, ["core", "all"], base.cpuScale),
    range: oneOf(raw?.range, RANGES, base.range),
    showVirtual: raw?.showVirtual === true,
    alerts: cleanAlerts(raw?.alerts),
    processes: {
      sort: typeof processes?.sort === "string" ? processes.sort : base.processes.sort,
      descending: processes?.descending !== false,
      filter: oneOf(processes?.filter, ["all", "gpu", "active"], base.processes.filter),
      group: processes?.group === true,
      columns: words(processes?.columns).length ? words(processes.columns) : base.processes.columns,
    },
    layout: cleanLayout(raw?.layout),
  };
}

function read() {
  try {
    const text = localStorage.getItem(KEY);
    return text ? JSON.parse(text) : null;
  } catch {
    return null;
  }
}

/** The live settings object, shared by every module. */
export const settings = clean(read());

export let storageWorks = true;

const listeners = new Set();
export const onSettingsChange = listener => listeners.add(listener);

/** Stores the settings and tells the page. */
export function saveSettings() {
  try {
    localStorage.setItem(KEY, JSON.stringify(settings));
    storageWorks = true;
  } catch {
    storageWorks = false;
  }
  for (const listener of listeners) listener();
}

/** Replaces all settings at once, as import and reset do. */
export function replaceSettings(raw) {
  Object.assign(settings, clean(raw));
  saveSettings();
}

/** The file a viewer can keep or carry to another browser. */
export function exportJson() {
  return JSON.stringify({ app: "baylee-pulse", version: 1, settings }, null, 2);
}

/** Validates an export; returns its settings or throws a sentence to show. */
export function parseImport(text) {
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error("Das ist kein gültiges JSON.");
  }
  if (parsed?.app !== "baylee-pulse" || typeof parsed.settings !== "object") {
    throw new Error("Das ist keine Pulse-Datei: Es fehlt die Kennung „baylee-pulse“.");
  }
  if (parsed.version !== 1) throw new Error(`Unbekannte Dateiversion ${parsed.version}.`);
  return clean(parsed.settings);
}

/** The settings of a fresh browser. */
export const freshSettings = () => clean(null);
