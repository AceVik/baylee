// Small formatting helpers, all pure.

/** `bytes` as a person reads it: 912 B, 21.1 KiB, 3.4 MiB. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KiB", "MiB", "GiB"] as const;
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value < 10 ? 1 : 0)} ${units[unit] ?? "GiB"}`;
}

/** An ISO time as `YYYY-MM-DD HH:MM` UTC; the text itself if it is not one. */
export function formatTime(iso: string): string {
  const match = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})/.exec(iso);
  return match ? `${match[1] ?? ""} ${match[2] ?? ""} UTC` : iso;
}

/** Milliseconds since the epoch as `HH:MM:SS` UTC; empty for 0 (undated). */
export function formatClock(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return "";
  return new Date(ms).toISOString().slice(11, 19);
}

/** A pseudonym short enough for a table cell; the full one goes in a title. */
export function shortPseudonym(reporter: string): string {
  return reporter.length > 10 ? `${reporter.slice(0, 10)}…` : reporter;
}

/** Only the base64 alphabet: what may go into a `data:` URL. */
export function isBase64(text: string): boolean {
  return text.length > 0 && text.length % 4 === 0 && /^[A-Za-z0-9+/]+={0,2}$/.test(text);
}
