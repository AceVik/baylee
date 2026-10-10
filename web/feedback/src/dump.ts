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

/** One card the player's text names in brackets (`client.refs.cards`). */
export interface CardRef {
  text: string;
  /** Where in the text, `[start, end)` in characters, brackets included. */
  at: [number, number] | null;
  card: number | null;
  zone: string | null;
  owner: number | null;
  /** The printing's Scryfall id; only one shaped like an id. */
  scryfallId: string | null;
  /** Scryfall's page for the printing; only for an id shaped like one. */
  scryfall: string | null;
}

/** One player the text names (`client.refs.players`): a seat, never an account. */
export interface PlayerRef {
  text: string;
  at: [number, number] | null;
  seat: number | null;
}

/** `[start, end)` as the client writes it, or nothing. */
function range(v: unknown): [number, number] | null {
  if (!Array.isArray(v) || v.length !== 2) return null;
  const [a, b] = v.map(num);
  return a != null && b != null ? [a, b] : null;
}

export interface Refs {
  cards: CardRef[];
  players: PlayerRef[];
}

const SCRYFALL_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/**
 * The references the client derived from the text (window B): what each
 * `[Name]` and `[@Name]` is, which the text alone cannot say. `null` when
 * the text names nothing.
 */
export function readRefs(client: Record<string, unknown>): Refs | null {
  const refs = part(client, "refs");
  if (!refs) return null;
  const cards = Array.isArray(refs["cards"]) ? refs["cards"] : [];
  const players = Array.isArray(refs["players"]) ? refs["players"] : [];
  const read: Refs = {
    cards: cards.filter(isRecord).map((c) => {
      const print = isRecord(c["print"]) ? c["print"] : null;
      const id = print ? text(print["scryfall_id"]) : null;
      const valid = id !== null && SCRYFALL_ID.test(id);
      return {
        text: text(c["text"]) ?? "?",
        at: range(c["at"]),
        card: num(c["card"]),
        zone: text(c["zone"]),
        owner: num(c["owner"]),
        scryfallId: valid ? id : null,
        scryfall: valid ? `https://scryfall.com/card/${id}` : null,
      };
    }),
    players: players.filter(isRecord).map((p) => ({
      text: text(p["text"]) ?? "?",
      at: range(p["at"]),
      seat: num(p["seat"]),
    })),
  };
  return read.cards.length + read.players.length > 0 ? read : null;
}
