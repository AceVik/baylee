/* How numbers read in German. Unknown stays a dash, never a zero. */

import { settings } from "./store.js";

const one = new Intl.NumberFormat("de-DE", { maximumFractionDigits: 1 });
const whole = new Intl.NumberFormat("de-DE", { maximumFractionDigits: 0 });
const two = new Intl.NumberFormat("de-DE", { maximumFractionDigits: 2 });

export const DASH = "—";
export const number = value => (value == null ? DASH : one.format(value));
export const integer = value => (value == null ? DASH : whole.format(value));
export const percent = value => (value == null ? DASH : `${one.format(value)} %`);
export const watts = value => (value == null ? DASH : `${one.format(value)} W`);
export const celsius = value => (value == null ? DASH : `${one.format(value)} °C`);
export const mhz = value => (value == null ? DASH : value >= 1000 ? `${two.format(value / 1000)} GHz` : `${whole.format(value)} MHz`);
export const perSecond = value => (value == null ? DASH : `${one.format(value)}/s`);

/** Scales `value` into the largest unit that keeps it at or above one. */
function scaled(value, base, units) {
  let at = 0;
  while (Math.abs(value) >= base && at < units.length - 1) {
    value /= base;
    at += 1;
  }
  return [value, units[at]];
}

const BINARY = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];

export function bytes(value) {
  if (value == null) return DASH;
  const [amount, unit] = scaled(value, 1024, BINARY);
  return `${one.format(amount)} ${unit}`;
}

/** A value and unit in the viewer's chosen data-rate unit. */
export function rateParts(value) {
  if (settings.rateUnit === "bits") return scaled(value * 8, 1000, ["bit/s", "kbit/s", "Mbit/s", "Gbit/s", "Tbit/s"]);
  if (settings.rateUnit === "si") return scaled(value, 1000, ["B/s", "kB/s", "MB/s", "GB/s", "TB/s"]);
  return scaled(value, 1024, ["B/s", "KiB/s", "MiB/s", "GiB/s", "TiB/s"]);
}

export function rate(value) {
  if (value == null) return DASH;
  const [amount, unit] = rateParts(value);
  return `${one.format(amount)} ${unit}`;
}

/** A link speed, as Ethernet states them. */
export function linkSpeed(mbps) {
  if (mbps == null) return DASH;
  return mbps >= 1000 ? `${one.format(mbps / 1000)} Gbit/s` : `${whole.format(mbps)} Mbit/s`;
}

export function pcieLink(link) {
  if (!link || (link.generation == null && link.width == null)) return DASH;
  const now = `PCIe ${link.generation ?? "?"}.0 ×${link.width ?? "?"}`;
  const reduced = (link.max_generation != null && link.max_generation > link.generation)
    || (link.max_width != null && link.max_width > link.width);
  return reduced ? `${now} (möglich: ${link.max_generation ?? "?"}.0 ×${link.max_width ?? "?"})` : now;
}

export const time = ms => new Date(ms).toLocaleTimeString("de-DE", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
export const shortTime = ms => new Date(ms).toLocaleTimeString("de-DE", { hour: "2-digit", minute: "2-digit" });

/** A span of time, as briefly as stays useful. */
export function span(ms) {
  if (ms == null || ms < 0) return DASH;
  const minutes = Math.floor(ms / 60000);
  if (minutes < 1) return `${Math.round(ms / 1000)} s`;
  if (minutes < 60) return `${minutes} Min.`;
  const hours = Math.floor(minutes / 60);
  return hours < 48 ? `${hours} Std. ${minutes % 60} Min.` : `${Math.floor(hours / 24)} T. ${hours % 24} Std.`;
}

/** The time of day, with the date only when it is not today. */
export function clock(ms, now) {
  const sameDay = new Date(ms).toDateString() === new Date(now).toDateString();
  return new Date(ms).toLocaleString("de-DE", sameDay ? { hour: "2-digit", minute: "2-digit" } : { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });
}

/** The smallest round number at or above `value`, for a graph's top line. */
export function niceCeiling(value) {
  if (!(value > 0)) return 1;
  const magnitude = 10 ** Math.floor(Math.log10(value));
  const step = [1, 1.5, 2, 2.5, 3, 4, 5, 6, 8, 10].find(candidate => candidate * magnitude >= value);
  return (step ?? 10) * magnitude;
}

/** The same in the scale of a data rate, so axis labels land on round numbers. */
export function niceRateCeiling(value) {
  if (!(value > 0)) return settings.rateUnit === "bits" ? 125 : 1024;
  const [amount] = rateParts(value);
  return value * (niceCeiling(amount) / amount);
}
