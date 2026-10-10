// The table the report was written at, read into a few lines an admin
// can take in without unfolding the whole `client.game` object
// (`crates/baylee-client-core/src/bugreport.rs`: `Table`, `Holding`, the
// engine's `Pending`).

import { isRecord } from "../api";
import { part } from "../dump";

const text = (v: unknown): string | null => (typeof v === "string" ? v : null);
const num = (v: unknown): number | null => (typeof v === "number" && Number.isFinite(v) ? v : null);

export interface Holding {
  selected: number | null;
  armed: string | null;
  manaRun: boolean;
  outbox: number | null;
  lastError: string | null;
}

export interface GameSummary {
  /** The reporter's chair, as the view counts them. */
  seat: number | null;
  /** The view's sequence number, to line up with the record. */
  seq: number | null;
  /** Turn, phase and step as the client spelled them. */
  when: string | null;
  /** The turn number, read off the view when it carries one. */
  turn: number | null;
  /** The phase and step as the view names them (`Main1`, `Upkeep`, …). */
  phase: string | null;
  step: string | null;
  /** Whose turn, as a seat number. */
  activeSeat: number | null;
  /** The seat the table waited for. */
  awaiting: number | null;
  /** The question the engine was asking: the variant's name, `null` for none. */
  pending: string | null;
  /** What the pending question carries, flattened to a line. */
  pendingDetail: string | null;
  holding: Holding | null;
  /** How many players the view counts. */
  players: number | null;
}

/**
 * The name of a `Pending` as serde writes an externally tagged enum: a
 * string for a unit variant, `{"Name": {...}}` for the rest.
 */
export function pendingName(value: unknown): string | null {
  if (typeof value === "string") return value;
  if (!isRecord(value)) return null;
  const keys = Object.keys(value);
  return keys.length === 1 ? (keys[0] ?? null) : null;
}

/** A few scalar fields of the question, as `key: value` pairs. */
function pendingDetail(value: unknown): string | null {
  if (!isRecord(value)) return null;
  const inner = Object.values(value)[0];
  if (!isRecord(inner)) return null;
  const pairs: string[] = [];
  for (const [key, v] of Object.entries(inner)) {
    if (typeof v === "number" || typeof v === "boolean") pairs.push(`${key}: ${String(v)}`);
    else if (typeof v === "string" && v.length <= 40) pairs.push(`${key}: ${v}`);
    else if (Array.isArray(v)) pairs.push(`${key}: ${v.length}`);
    if (pairs.length >= 4) break;
  }
  return pairs.length === 0 ? null : pairs.join(" · ");
}

/** A short word for the variant, from `ChooseAttackers` to `Choose attackers`. */
export function pendingLabel(name: string): string {
  return name.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/^./, (c) => c.toUpperCase()).replaceAll(/\s[A-Z]/g, (m) => m.toLowerCase());
}

export function readGame(client: Record<string, unknown>): GameSummary | null {
  const game = part(client, "game");
  if (!game) return null;
  const table = isRecord(game["table"]) ? game["table"] : null;
  const view = isRecord(game["view"]) ? game["view"] : null;
  const holdingRaw = isRecord(game["holding"]) ? game["holding"] : null;
  const players = view && Array.isArray(view["players"]) ? view["players"].length : null;
  return {
    seat: table ? num(table["seat"]) : null,
    seq: table ? num(table["seq"]) : null,
    when: table ? text(table["when"]) : null,
    turn: view ? num(view["turn"]) : null,
    phase: view ? pendingName(view["phase"]) : null,
    step: view ? pendingName(view["step"]) : null,
    activeSeat: view ? num(view["active"]) : null,
    awaiting: view ? num(view["awaiting"]) : null,
    pending: pendingName(game["pending"]),
    pendingDetail: pendingDetail(game["pending"]),
    holding: holdingRaw
      ? {
          selected: num(holdingRaw["selected"]),
          armed: text(holdingRaw["armed"]),
          manaRun: holdingRaw["mana_run"] === true,
          outbox: num(holdingRaw["outbox"]),
          lastError: text(holdingRaw["last_error"]),
        }
      : null,
    players,
  };
}
