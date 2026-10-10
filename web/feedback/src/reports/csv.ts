// A listing as CSV, made in the browser from what the list already holds.

import type { Summary } from "../api";

/** `value` as one CSV field: quoted when it holds a comma, a quote or a line break. */
export function field(value: string | number | boolean | null): string {
  const text = value === null ? "" : String(value);
  return /[",\r\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

export const COLUMNS = [
  "id",
  "created_at",
  "updated_at",
  "kind",
  "status",
  "gateway",
  "channel",
  "reporter",
  "game_id",
  "has_record",
  "record_bytes",
  "issue_number",
  "text",
] as const;

/** `reports` as CSV with a header line, `\r\n` line ends, UTF-8 without a BOM. */
export function toCsv(reports: readonly Summary[]): string {
  const lines = [COLUMNS.join(",")];
  for (const r of reports) {
    lines.push(
      [
        r.id,
        r.created_at,
        r.updated_at,
        r.kind,
        r.status,
        r.channel === "direct" ? "(direct)" : r.gateway,
        r.channel,
        r.reporter,
        r.game_id,
        r.has_record,
        r.record_bytes,
        r.issue_number,
        r.text,
      ]
        .map(field)
        .join(","),
    );
  }
  return `${lines.join("\r\n")}\r\n`;
}

/** Hands `text` to the browser as a file called `name`. */
export function download(name: string, text: string, type = "text/csv;charset=utf-8"): void {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => {
    URL.revokeObjectURL(url);
  }, 1000);
}
