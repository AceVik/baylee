/* Applies theme, accent and density, and hands canvas code the live colours. */

import { settings, onSettingsChange } from "./store.js";

const cache = new Map();
const listeners = new Set();

/** A CSS custom property's current value, for drawing on a canvas. */
export function token(name) {
  let value = cache.get(name);
  if (value === undefined) {
    value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    cache.set(name, value);
  }
  return value;
}

export const onThemeChange = listener => listeners.add(listener);

function applied() {
  const root = document.documentElement;
  if (settings.theme === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", settings.theme);
  root.setAttribute("data-accent", settings.accent);
  root.setAttribute("data-density", settings.density);
  const dark = settings.theme === "dark" || (settings.theme === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", dark ? "#0b1117" : "#eef2f6");
  cache.clear();
  for (const listener of listeners) listener();
}

export function initTheme() {
  applied();
  onSettingsChange(applied);
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applied);
}
