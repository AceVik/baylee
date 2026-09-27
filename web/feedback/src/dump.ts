// Reading the parts of a report's `client` object the client writes
// (crates/baylee-client-core/src/bugreport.rs). Each part is optional: the
// player ticks what goes in. A part that is missing or not the shape
// expected reads as `null`, and the detail view says so instead of breaking.

import { isRecord } from "./api";
import { formatClock, isBase64 } from "./format";

const text = (v: unknown): string | null => (typeof v === "string" ? v : null);
const num = (v: unknown): number | null => (typeof v === "number" && Number.isFinite(v) ? v : null);

export function part(client: Record<string, unknown>, key: string): Record<string, unknown> | null {
  const value = client[key];
  return isRecord(value) ? value : null;
}

export interface Build {
  version: string | null;
  commit: string | null;
}

export function readBuild(client: Record<string, unknown>): Build | null {
  const build = part(client, "build");
  if (!build) return null;
  return { version: text(build["version"]), commit: text(build["commit"]) };
}

/** The system part as label and value, in the order the form shows them. */
export function readSystem(client: Record<string, unknown>): [string, string][] | null {
  const system = part(client, "system");
  if (!system) return null;
  const rows: [string, string][] = [];
  const add = (label: string, value: string | null) => {
    if (value !== null && value !== "") rows.push([label, value]);
  };
  add("Platform", text(system["platform"]));
  add("CPUs", num(system["cpus"])?.toString() ?? null);
  add("Graphics adapter", text(system["adapter"]));
  add("Graphics backend", text(system["backend"]));
  const window = system["window"];
  if (Array.isArray(window) && window.length === 2) {
    const [w, h] = window.map(num);
    if (w != null && h != null) add("Window", `${w} × ${h}`);
  }
  add("Scale", num(system["scale"])?.toString() ?? null);
  add("Language", text(system["lang"]));
  return rows;
}

export interface Screenshot {
  width: number | null;
  height: number | null;
  /** A `data:image/png;base64,…` URL, made only from the base64 alphabet. */
  src: string;
}

export function readScreenshot(client: Record<string, unknown>): Screenshot | null {
  const shot = part(client, "screenshot");
  if (!shot) return null;
  const data = text(shot["png_base64"]);
  if (data === null || !isBase64(data)) return null;
  return {
    width: num(shot["width"]),
    height: num(shot["height"]),
    src: `data:image/png;base64,${data}`,
  };
}

export interface LogLine {
  turn: number | null;
  clock: string;
  text: string;
}

export interface SeatLog {
  seat: number | null;
  roster: { seat: number | null; name: string; isAi: boolean; team: number | null }[];
  lines: LogLine[];
}

export function readLog(client: Record<string, unknown>): SeatLog | null {
  const log = part(client, "log");
  if (!log) return null;
  const roster = Array.isArray(log["roster"]) ? log["roster"] : [];
  const lines = Array.isArray(log["lines"]) ? log["lines"] : [];
  return {
    seat: num(log["seat"]),
    roster: roster.filter(isRecord).map((r) => ({
      seat: num(r["seat"]),
      name: text(r["name"]) ?? "?",
      isAi: r["is_ai"] === true,
      team: num(r["team"]),
    })),
    lines: lines.filter(isRecord).map((l) => ({
      turn: num(l["turn"]),
      clock: formatClock(num(l["at"]) ?? 0),
      text: text(l["text"]) ?? "",
    })),
  };
}

export interface Crash {
  message: string;
  location: string | null;
  backtrace: string | null;
  at: string | null;
  platform: string | null;
  thread: string | null;
}

export function readCrash(client: Record<string, unknown>): Crash | null {
  const crash = part(client, "crash");
  if (!crash) return null;
  const at = num(crash["at_unix"]);
  return {
    message: text(crash["message"]) ?? "(no message)",
    location: text(crash["location"]),
    backtrace: text(crash["backtrace"]),
    at: at !== null && at > 0 ? new Date(at * 1000).toISOString() : null,
    platform: text(crash["platform"]),
    thread: text(crash["thread"]),
  };
}
