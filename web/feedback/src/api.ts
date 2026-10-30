// The service's `/ui/api` routes (docs/feedback.md §"The web UI"). Every
// request is same-origin and carries the session cookie; every change also
// carries `X-Baylee-CSRF: 1`, which a form on another site cannot send.

export const KINDS = ["bug", "improvement", "feedback", "crash", "other", "chat"] as const;
export type Kind = (typeof KINDS)[number];

export const STATUSES = [
  "new",
  "triaged",
  "in_progress",
  "resolved",
  "wont_fix",
  "duplicate",
] as const;
export type Status = (typeof STATUSES)[number];

/** The statuses still to be dealt with, and the ones that are done. */
export const OPEN_STATUSES: readonly Status[] = ["new", "triaged", "in_progress"];
export const DONE_STATUSES: readonly Status[] = ["resolved", "wont_fix", "duplicate"];

/** How urgent a report is, most urgent first. */
export const PRIORITIES = ["critical", "high", "normal", "low"] as const;
export type Priority = (typeof PRIORITIES)[number];

/** What the list can be sorted by (`crate::triage`'s `Sort`). */
export const SORTS = ["received", "changed", "priority", "status", "kind", "reporter", "gateway"] as const;
export type Sort = (typeof SORTS)[number];

/** The page sizes on offer; the service takes at most 200. */
export const PAGE_SIZES = [25, 50, 100, 200] as const;

/** How long a report stays in the trash (`crate::triage::TRASH_DAYS`). */
export const TRASH_DAYS = 30;

export const STATUS_LABELS: Record<Status, string> = {
  new: "New",
  triaged: "Triaged",
  in_progress: "In progress",
  resolved: "Resolved",
  wont_fix: "Won't fix",
  duplicate: "Duplicate",
};

export interface Summary {
  id: string;
  created_at: string;
  updated_at: string;
  gateway: string;
  gateway_name: string | null;
  gateway_url: string | null;
  gateway_version: string;
  reporter: string;
  kind: Kind;
  status: Status;
  text: string;
  game_id: string | null;
  has_record: boolean;
  record_complete: boolean | null;
  record_bytes: number;
  /** How it came: passed on by a gateway, or sent by a client itself, unauthenticated. */
  channel: "gateway" | "direct";
  /** Who wrote the record: a gateway, or a client (unverified); null without one. */
  record_origin: "gateway" | "client" | null;
  issue_number: number | null;
  issue_url: string | null;
  /** The admin who looks after it. */
  assignee: string | null;
  priority: Priority | null;
  labels: string[];
  /** The report this one duplicates. */
  duplicate_of: string | null;
  /** A crash's signature: the same crash, numbers aside, has the same one. */
  signature: string | null;
  /** Moved on by every change; a change names the one it was made against. */
  revision: number;
  /** When it entered a done status. */
  done_at: string | null;
  /** When it went into the trash. */
  deleted_at: string | null;
  /** Whether the signed-in admin has opened it. */
  seen: boolean;
  /** How many internal notes it has. */
  notes: number;
}

/** An internal note: an admin's, about a report, never shown to a player. */
export interface Note {
  id: string;
  at: string;
  author: string;
  text: string;
}

export interface Report extends Summary {
  client: Record<string, unknown>;
  /** Its notes, oldest first (the UI's routes only). */
  note_list?: Note[];
}

/** What a change sets; a part left out stays, `null` clears it. */
export interface Change {
  status?: Status;
  issue?: number | null;
  assignee?: string | null;
  priority?: Priority | null;
  labels?: string[];
  duplicate_of?: string | null;
  /** The revision the change was made against; another one is refused (409). */
  expected_revision?: number;
}

/** A change for many reports: labels are added and removed, not replaced. */
export interface BulkChange {
  status?: Status;
  issue?: number | null;
  assignee?: string | null;
  priority?: Priority | null;
  labels_add?: string[];
  labels_remove?: string[];
  duplicate_of?: string | null;
}

export type BulkAction = "trash" | "restore" | "purge";

/** One crash, grouped by signature, under the list's filter. */
export interface CrashGroup {
  signature: string;
  count: number;
  open: number;
  first: string;
  last: string;
  latest: string;
  message: string | null;
  location: string | null;
}

/** Where a report stands in the list a filter makes. */
export interface Neighbours {
  previous: string | null;
  next: string | null;
  position: number | null;
  total: number;
}

/** A list an admin saved: a name and the list's query string. */
export interface SavedView {
  id: string;
  name: string;
  query: string;
}

/** A message from the landing page's contact form (`docs/feedback.md`). */
export interface ContactMessage {
  id: string;
  created_at: string;
  updated_at: string;
  name: string | null;
  email: string | null;
  message: string;
  language: "en" | "de";
  status: ContactStatus;
  done_at: string | null;
}

export const CONTACT_STATUSES = ["new", "done"] as const;
export type ContactStatus = (typeof CONTACT_STATUSES)[number];

/** `GET /ui/api/contact`: the newest messages the filter matches. */
export interface ContactListing {
  open: number;
  total: number;
  messages: ContactMessage[];
}

/** `GET /ui/api/contact/{id}`: one message and what was done with it. */
export interface ContactDetail extends ContactMessage {
  audit: AuditEntry[];
}

export interface Listing {
  total: number;
  reports: Summary[];
}

/** One value of a facet; `null` for nobody assigned or no priority. */
export interface Count {
  value: string | null;
  count: number;
}

/** The counts beside the list, each under the filter but its own dimension. */
export interface Facets {
  gateways: Count[];
  statuses: Count[];
  kinds: Count[];
  reporters: Count[];
  assignees: Count[];
  priorities: Count[];
  labels: Count[];
  channels: Count[];
  total: number;
}

export interface AuditEntry {
  at: string;
  actor: string;
  action: string;
  detail: string | null;
}

export interface Me {
  name: string;
  /** Whether a gateway's admin console is configured (the Overview pages). */
  gateway_admin?: boolean;
}

/** How many since three moments (`docs/protocol.md` §"The admin console"). */
export interface Since {
  today_utc: number;
  last_7d: number;
  last_30d: number;
}

/** One UTC day of the overview's series. */
export interface Day {
  day: string;
  registered: number;
  guests: number;
  started: number;
  finished: number;
  players: number;
}

/** `GET /ui/api/admin/offline`: offline games going now, anonymous. */
export interface Offline {
  offline_now: number;
  window_secs: number;
}

/** `GET /ui/api/admin/stats`: a gateway's numbers, counts only. */
export interface Stats {
  at: string;
  gateway: {
    name: string | null;
    version: string;
    registration: string;
    commit?: string;
    built_at?: string;
    uptime_secs?: number;
    terms?: boolean;
    mail?: boolean;
  };
  accounts: {
    registered: number;
    with_email: number;
    confirmed_email: number;
    admitted_by_key: number;
    created: Since;
    decks?: number;
  };
  guests: { enabled: boolean; live: number; cap: number | null; created?: Since };
  online: {
    players: number;
    in_lobby: number;
    seated: number;
    sessions_live: number;
    accounts_signed_in: number;
  };
  games: {
    running: number;
    local_running: number;
    waiting: number;
    seats_awaiting_engine: number;
    recorded: number;
    started: Since;
    finished: number;
    finished_since: Since;
    record_bytes?: number;
    avg_secs_30d?: number;
  };
  agents: { connected: number; local: number; capacity: number | null; games: number };
  invites: {
    total: number;
    active: number;
    used_up: number;
    expired: number;
    revoked: number;
    uses_left: number;
    admitted: number;
  };
  daily?: Day[];
}

/** Someone online now, as the gateway's memory says. */
export interface OnlinePlayer {
  id: string;
  handle: string | null;
  guest: boolean | null;
  in_lobby: boolean;
  /** The running game they sit in. */
  playing: string | null;
  /** The waiting room they sit in. */
  waiting: string | null;
}

export interface LiveSeat {
  seat: number;
  kind: "human" | "ai";
  ai: string | null;
  account_id: string | null;
  player: string | null;
  guest: boolean | null;
  bridge: string | null;
  bridged_by: string | null;
  deck: string;
  format: string | null;
  ready: boolean;
  team: number | null;
}

export interface LiveTable {
  id: string;
  name: string;
  state: "waiting" | "playing";
  host: string | null;
  host_id: string | null;
  created_at: string;
  locked: boolean;
  rematch: boolean;
  decide_secs: number;
  engine: boolean;
  engine_local: boolean;
  agent: string | null;
  seats: LiveSeat[];
}

export interface LiveAgent {
  id: string;
  name: string;
  local: boolean;
  capacity: number;
  games: number;
}

/** `GET /ui/api/admin/live`. */
export interface Live {
  at: string;
  players: OnlinePlayer[];
  tables: LiveTable[];
  agents: LiveAgent[];
}

/** One engine process in a sample of the server metrics. */
export interface GameSample {
  game_id: string;
  pid: number;
  /** Cores used over the interval; null on a process's first sample. */
  cpu: number | null;
  /** Resident memory in bytes. */
  rss: number;
}

/** One point of the server metrics; `null` where the host could not say. */
export interface MetricSample {
  at: number;
  cpu: number | null;
  load1: number | null;
  mem_used: number | null;
  disk_used: number | null;
  net_in: number | null;
  net_out: number | null;
  sockets: number | null;
  requests: number;
  games: GameSample[];
}

/** `GET /ui/api/admin/metrics`: the last hour of the gateway's machine. */
export interface Metrics {
  at: string;
  interval_secs: number;
  keep: number;
  platform: string;
  cores: number;
  process_uptime_secs: number;
  host: {
    uptime_secs: number | null;
    mem_total: number | null;
    disk_total: number | null;
    disk_path: string;
    load: [number, number, number] | null;
  };
  now: MetricSample | null;
  history: {
    at: number[];
    cpu: (number | null)[];
    load1: (number | null)[];
    mem_used: (number | null)[];
    disk_used: (number | null)[];
    net_in: (number | null)[];
    net_out: (number | null)[];
    sockets: (number | null)[];
    requests: number[];
  };
  games: { game_id: string; pid: number; cpu: (number | null)[]; rss: (number | null)[] }[];
}

/**
 * `generated`: read in full by codegen, playable, but no card test plays it
 * yet. Counted apart from `implemented`.
 */
export const COVERAGES = ["implemented", "generated", "partial", "unimplemented", "absent"] as const;
export type Coverage = (typeof COVERAGES)[number];

/** One set's progress: the cards first printed in it, by how far each is. */
export interface SetProgress {
  code: string;
  position: number;
  total: number;
  implemented: number;
  generated: number;
  partial: number;
  unimplemented: number;
  absent: number;
}

/** `GET /ui/api/admin/sets`. */
export interface SetsOverview {
  at: string;
  version: string;
  corpus: number;
  pool: { cards: number; implemented: number; generated: number; partial: number; unimplemented: number; absent: number };
  sets: SetProgress[];
}

/** One card of a set. */
export interface SetCard {
  index: number;
  name: string;
  coverage: Coverage;
  note: string | null;
  type_line: string;
  scryfall_id: string | null;
  oracle_id: string;
}

/**
 * A gateway older than the `generated` tier sends no such count; the
 * service is deployed before the gateway, so it reads as zero.
 */
export function withGenerated<T extends { generated?: number }>(counts: T): T & { generated: number } {
  return { ...counts, generated: counts.generated ?? 0 };
}

/** `GET /ui/api/admin/sets/{code}`. */
export interface SetDetail extends SetProgress {
  cards: SetCard[];
}

/** `GET /ui/api/stats`: reports per UTC day and kind, the last 30 days. */
export interface ReportStats {
  days: number;
  rows: { day: string; kind: string; count: number }[];
}

/** `GET /health` of the service itself: no session needed. */
export interface Health {
  ok: boolean;
  version: string;
  source: string;
}

export const ACCOUNT_KINDS = ["all", "registered", "guest"] as const;
export type AccountKind = (typeof ACCOUNT_KINDS)[number];
export const ACCOUNT_SORTS = ["newest", "oldest", "name", "games", "active"] as const;
export type AccountSort = (typeof ACCOUNT_SORTS)[number];

/** One account as the console lists it: never a secret or an address. */
export interface AccountRow {
  id: string;
  username: string | null;
  display_name: string;
  tag: number;
  handle: string;
  guest: boolean;
  created_at: string;
  has_email: boolean;
  confirmed: boolean;
  lang: string;
  by_key: boolean;
  key_note: string | null;
  terms_version: string | null;
  decks: number;
  games: number;
  sessions: number;
  active_at: string | null;
  in_lobby: boolean;
  playing: boolean;
  online: boolean;
  /** An admin reset its password and it has not chosen its own yet. */
  reset_pending?: boolean;
  /** The ban in force, if one is. */
  ban?: AccountBan | null;
}

/** A ban in force on an account (`docs/protocol.md` §"Acting on an account"). */
export interface AccountBan {
  reason: string;
  /** RFC 3339, UTC; null for good. */
  until: string | null;
  at: string;
  by: string;
}

export interface AccountPage {
  total: number;
  online: number;
  accounts: AccountRow[];
}

export interface AccountFilter {
  q?: string;
  /** One exact tag in hex (`af03`): the sender a chat report names. */
  tag?: string;
  kind?: AccountKind;
  sort?: AccountSort;
  online?: boolean;
  offset?: number;
}

export const ACCOUNT_PAGE = 50;

export interface AccountDeck {
  id: string;
  name: string;
  format: string;
  cards: number;
  version: number;
  updated_at: string;
}

export interface AccountGame {
  game_id: string;
  started_at: string;
  ended_at: string | null;
  complete: boolean;
  seat: number;
  chairs: { seat: number; player: string | null; guest: boolean | null }[];
}

export interface AccountDetail extends Omit<AccountRow, "playing"> {
  terms_accepted_at: string | null;
  confirmed_at: string | null;
  deck_list: AccountDeck[];
  recent_games: AccountGame[];
  table: { id: string; name: string; state: "waiting" | "playing" } | null;
}

/** An account search as the query string carries it. */
export function accountQuery(filter: AccountFilter): string {
  const params = new URLSearchParams();
  const q = filter.q?.trim();
  if (q) params.set("q", q);
  const tag = senderTag(filter.tag);
  if (tag !== null) params.set("tag", tag);
  if (filter.kind && filter.kind !== "all") params.set("kind", filter.kind);
  if (filter.sort && filter.sort !== "newest") params.set("sort", filter.sort);
  if (filter.online) params.set("online", "true");
  if (filter.offset !== undefined && filter.offset > 0) params.set("offset", String(filter.offset));
  return params.toString();
}

/**
 * A tag as the gateway draws it (lowercase hex, `#` and a name before it
 * dropped), or `null` for anything that is not one. A chat report's
 * `client.chat.sender_tag` and the accounts filter both go through here.
 */
export function senderTag(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const bare = value.trim().split("#").pop()?.toLowerCase() ?? "";
  return /^[0-9a-f]{1,8}$/.test(bare) ? bare : null;
}

/** An account search read back from a query string. */
export function parseAccountFilter(search: string): AccountFilter {
  const params = new URLSearchParams(search);
  const filter: AccountFilter = {};
  const q = params.get("q");
  if (q) filter.q = q;
  const tag = senderTag(params.get("tag"));
  if (tag !== null) filter.tag = tag;
  const kind = oneOf(ACCOUNT_KINDS, params.get("kind"));
  if (kind) filter.kind = kind;
  const sort = oneOf(ACCOUNT_SORTS, params.get("sort"));
  if (sort) filter.sort = sort;
  if (params.get("online") === "true") filter.online = true;
  const offset = Number(params.get("offset"));
  if (Number.isInteger(offset) && offset > 0) filter.offset = offset;
  return filter;
}

export const INVITE_STATES = ["active", "used_up", "expired", "revoked"] as const;
export type InviteState = (typeof INVITE_STATES)[number];

/** A closed-beta key as the gateway lists it: never the key. */
export interface Invite {
  id: string;
  created_at: string;
  note: string | null;
  uses_left: number;
  expires_at: string | null;
  revoked_at: string | null;
  admitted: number;
  state: InviteState;
}

/** The gateway's link to Baylee Central (`GET /admin/central`); never a token, only its last four characters. */
export interface CentralLink {
  source: "file" | "token" | "none";
  token: { last4: string; set_at: string; set_by: string } | null;
  central: {
    gateway: string;
    origin: string;
    name: string;
    state: "pending" | "approved" | "disabled";
    sso: boolean;
    listing: boolean;
    expires_at: string;
  } | null;
  check: "ok" | "rejected" | "unavailable" | null;
}

/** What `invite create` takes, as fields. */
export interface InviteOrder {
  count?: number;
  uses?: number;
  expires?: string;
  note?: string;
}

/** Keys just made: the only time a key is shown. */
export interface MadeKeys {
  keys: { id: string; key: string }[];
  uses: number;
  expires_at: string | null;
  note: string | null;
}

/** A card's block as the gateway tells it: `until` null for no end. */
export interface BlockNote {
  index: number;
  name: string;
  reason: string;
  until: string | null;
}

/** A card of the gateway's pool, found by name. */
export interface CardHit {
  index: number;
  name: string;
  type_line: string;
  mana_cost: string;
  coverage: string;
  scryfall_id: string;
  blocked: BlockNote | null;
}

export const CARD_BLOCK_STATES = ["active", "expired", "lifted"] as const;
export type CardBlockState = (typeof CARD_BLOCK_STATES)[number];

/** A block the console made, in force or not any more. */
export interface CardBlock {
  id: string;
  index: number;
  name: string;
  reason: string;
  created_at: string;
  created_by: string;
  until: string | null;
  lifted_at: string | null;
  lifted_by: string | null;
  state: CardBlockState;
}

/** What blocking a card takes: `until` an ISO moment, absent for no end. */
export interface BlockOrder {
  index: number;
  reason: string;
  until?: string;
}

/** Who may seat a hosted profile (docs/protocol.md §"Who may seat which model"). */
export const LLM_POLICIES = ["registered", "listed", "nobody"] as const;
export type LlmPolicy = (typeof LLM_POLICIES)[number];

/** One profile's policy, as `GET /ui/api/admin/llm/access` lists it. */
export interface LlmAccess {
  id: string;
  policy: LlmPolicy;
  /** False: no policy is stored, and the profile is open to nobody. */
  stored: boolean;
  default_for_registered: boolean;
  /** How many accounts are on its list. */
  listed: number;
  updated_at: string | null;
  updated_by: string | null;
  /** Whether a connected seat agent reports it now. */
  connected: boolean;
  label: string | null;
  vendor: string | null;
}

/** An account on a profile's list. */
export interface LlmAllowed {
  id: string;
  username: string | null;
  display_name: string;
  handle: string;
  added_at: string;
  added_by: string;
}

/** One profile's policy with its list (`GET …/llm/access/{id}`). */
export interface LlmAccessDetail extends LlmAccess {
  accounts: LlmAllowed[];
}

/** A registered account the type-ahead found. */
export interface AccountCandidate {
  id: string;
  username: string | null;
  display_name: string;
  handle: string;
}

/** The fewest characters the account type-ahead asks the gateway for. */
export const LOOKUP_MIN = 2;

/** A hosted profile's states, best first, as the player list orders them. */
export const LLM_STATES = [
  "available",
  "busy",
  "probing",
  "exhausted",
  "needs_login",
  "failing",
  "disabled",
] as const;
export type LlmState = (typeof LLM_STATES)[number];

export interface LlmCaps {
  day_usd?: number;
  month_usd?: number;
  day_tokens?: number;
  month_tokens?: number;
}

/** What the gateway takes to write a profile (docs/protocol.md §"Hosted language-model seats"). */
export interface LlmDefinition {
  label: string;
  vendor: string;
  enabled?: boolean;
  max_games?: number;
  caps?: LlmCaps;
  canary?: boolean;
  /** Another API profile of the same seat agent (docs/protocol.md §"A fallback profile"). */
  fallback?: string;
  profile: Record<string, unknown>;
}

export interface LlmSpent {
  day_usd: number;
  month_usd: number;
  day_tokens: number;
  month_tokens: number;
}

/** One profile as one seat agent reported it. */
export interface LlmProfile {
  id: string;
  label: string;
  vendor: string;
  kind: "api" | "cli";
  model: string;
  state: LlmState;
  until_unix: number | null;
  games: number;
  max_games: number | null;
  enabled: boolean;
  canary: boolean;
  caps: LlmCaps | null;
  spent: LlmSpent | null;
  key: "kept" | "absent" | "none_needed" | "unavailable";
  last_error: string | null;
  last_ok_unix: number | null;
  /** The fallback new games start on now, while this profile is full or limited. */
  diverts?: string;
  definition: LlmDefinition;
}

/** A connected seat agent. */
export interface SeatHost {
  name: string;
  local: boolean;
  capacity: number;
  games: number;
  connected_secs: number;
  profiles: LlmProfile[];
}

/** A change made through the console, as this service audited it. */
export interface ConsoleChange {
  at: string;
  actor: string;
  action: string;
  detail: string | null;
}

/** The list's filters, as the query string carries them. */
export interface Filter {
  kind?: Kind[];
  status?: Status[];
  /** `none` is a report without a priority. */
  priority?: (Priority | "none")[];
  gateway?: string;
  reporter?: string;
  game_id?: string;
  q?: string;
  from?: string;
  to?: string;
  has_record?: boolean;
  channel?: "gateway" | "direct";
  /** An admin's name, or `none`. */
  assignee?: string;
  label?: string;
  signature?: string;
  duplicate_of?: string;
  /** The trash instead of the reports. */
  trash?: boolean;
  /** Only what this admin has not opened. */
  unseen?: boolean;
  // Page, size and order may be named `undefined`, to drop them from a copy.
  sort?: Sort | undefined;
  dir?: "asc" | "desc" | undefined;
  limit?: number | undefined;
  offset?: number | undefined;
}

export const PAGE_SIZE = 50;

/** Thrown for a 401: the session is gone, and the UI goes back to sign-in. */
export class SignedOut extends Error {
  constructor() {
    super("signed out");
    this.name = "SignedOut";
  }
}

/** Any other refusal, with the service's own sentence. */
export class Refused extends Error {
  readonly status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = "Refused";
    this.status = status;
  }
}

type Listener = () => void;
const signedOutListeners = new Set<Listener>();

/** Calls `listener` whenever a request finds the session gone. */
export function onSignedOut(listener: Listener): () => void {
  signedOutListeners.add(listener);
  return () => {
    signedOutListeners.delete(listener);
  };
}

export async function request(
  method: "GET" | "POST" | "PATCH" | "PUT" | "DELETE",
  path: string,
  body?: unknown,
): Promise<Response> {
  const headers: Record<string, string> = { Accept: "application/json" };
  if (method !== "GET") {
    headers["X-Baylee-CSRF"] = "1";
  }
  const init: RequestInit = { method, headers, credentials: "same-origin" };
  if (body !== undefined) {
    headers["Content-Type"] = "application/json";
    init.body = JSON.stringify(body);
  }
  const response = await fetch(path, init);
  if (response.status === 401 && path !== "/ui/api/login") {
    for (const listener of signedOutListeners) listener();
    throw new SignedOut();
  }
  if (!response.ok) {
    let message = `${response.status} ${response.statusText}`;
    try {
      const refusal: unknown = await response.json();
      if (isRecord(refusal) && typeof refusal["error"] === "string") {
        message = refusal["error"];
      }
    } catch {
      // Not JSON: the status line says it.
    }
    throw new Refused(response.status, message);
  }
  return response;
}

async function json<T>(method: "GET" | "POST" | "PATCH" | "PUT", path: string, body?: unknown): Promise<T> {
  const response = await request(method, path, body);
  return (await response.json()) as T;
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** `filter` as the query string the list route reads, and the UI's URL too. */
export function filterQuery(filter: Filter): string {
  const params = new URLSearchParams();
  const put = (key: string, value: string | number | boolean | undefined) => {
    if (value !== undefined && value !== "") params.set(key, String(value));
  };
  const list = (key: string, values: readonly string[] | undefined) => {
    if (values !== undefined && values.length > 0) params.set(key, values.join(","));
  };
  if (filter.trash === true) put("trash", true);
  list("status", filter.status);
  list("kind", filter.kind);
  list("priority", filter.priority);
  put("assignee", filter.assignee);
  put("label", filter.label);
  put("gateway", filter.gateway);
  put("channel", filter.channel);
  put("reporter", filter.reporter);
  put("game_id", filter.game_id);
  put("signature", filter.signature);
  put("duplicate_of", filter.duplicate_of);
  put("q", filter.q?.trim());
  put("from", filter.from);
  put("to", filter.to);
  put("has_record", filter.has_record);
  if (filter.unseen === true) put("unseen", true);
  if (filter.sort !== undefined && filter.sort !== "received") put("sort", filter.sort);
  if (filter.dir === "asc") put("dir", "asc");
  if (filter.limit !== undefined && filter.limit !== PAGE_SIZE) put("limit", filter.limit);
  if (filter.offset !== undefined && filter.offset > 0) put("offset", filter.offset);
  return params.toString();
}

/** `filter` without its page, size and order: what the facets and groups count over. */
export function scopeOf(filter: Filter): Filter {
  const { offset: _offset, limit: _limit, sort: _sort, dir: _dir, ...scope } = filter;
  return scope;
}

function oneOf<T extends string>(list: readonly T[], value: string | null): T | undefined {
  return list.find((item) => item === value);
}

/** The members of a comma list that are in `list`, once each, in `list`'s order. */
function someOf<T extends string>(list: readonly T[], value: string | null): T[] | undefined {
  if (value === null) return undefined;
  const wanted = new Set(value.split(","));
  const found = list.filter((item) => wanted.has(item));
  return found.length > 0 ? found : undefined;
}

const DAY = /^\d{4}-\d{2}-\d{2}$/;
const LABEL = /^[a-z0-9][a-z0-9-]{0,31}$/;
const SIGNATURE = /^[0-9a-f]{16}$/;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/** Whether `text` is a report's id as the service writes one. */
export function isReportId(text: string): boolean {
  return UUID.test(text);
}

/** A filter read back from a query string; anything malformed is dropped. */
export function parseFilter(search: string): Filter {
  const params = new URLSearchParams(search);
  const filter: Filter = {};
  const kind = someOf(KINDS, params.get("kind"));
  if (kind) filter.kind = kind;
  const status = someOf(STATUSES, params.get("status"));
  if (status) filter.status = status;
  const priority = someOf([...PRIORITIES, "none"] as const, params.get("priority"));
  if (priority) filter.priority = priority;
  for (const key of ["gateway", "reporter", "q", "game_id", "assignee"] as const) {
    const value = params.get(key);
    if (value) filter[key] = value;
  }
  const label = params.get("label");
  if (label !== null && LABEL.test(label)) filter.label = label;
  const signature = params.get("signature");
  if (signature !== null && SIGNATURE.test(signature)) filter.signature = signature;
  const original = params.get("duplicate_of");
  if (original !== null && UUID.test(original)) filter.duplicate_of = original;
  const channel = oneOf(["gateway", "direct"] as const, params.get("channel"));
  if (channel) filter.channel = channel;
  for (const key of ["from", "to"] as const) {
    const value = params.get(key);
    if (value && DAY.test(value)) filter[key] = value;
  }
  const record = params.get("has_record");
  if (record === "true" || record === "false") filter.has_record = record === "true";
  if (params.get("trash") === "true") filter.trash = true;
  if (params.get("unseen") === "true") filter.unseen = true;
  const sort = oneOf(SORTS, params.get("sort"));
  if (sort && sort !== "received") filter.sort = sort;
  if (params.get("dir") === "asc") filter.dir = "asc";
  const limit = Number(params.get("limit"));
  if (PAGE_SIZES.some((size) => size === limit) && limit !== PAGE_SIZE) filter.limit = limit;
  const offset = Number(params.get("offset"));
  if (Number.isInteger(offset) && offset > 0) filter.offset = offset;
  return filter;
}

/** `/ui/api/<route>` with `filter` as its query, an empty one left out. */
function withFilter(route: string, filter: Filter): string {
  const query = filterQuery(filter);
  return query ? `${route}?${query}` : route;
}

const reportPath = (id: string) => `/ui/api/reports/${encodeURIComponent(id)}`;

function listOf<T>(value: unknown): T[] {
  return Array.isArray(value) ? (value as T[]) : [];
}

/** Facets as the page reads them: a dimension the answer lacks (an older service's) counts nothing. */
function facetsOf(raw: unknown): Facets {
  const r = isRecord(raw) ? raw : {};
  const counts = (key: string) => listOf<Count>(r[key]);
  return {
    gateways: counts("gateways"),
    statuses: counts("statuses"),
    kinds: counts("kinds"),
    reporters: counts("reporters"),
    assignees: counts("assignees"),
    priorities: counts("priorities"),
    labels: counts("labels"),
    channels: counts("channels"),
    total: typeof r["total"] === "number" ? r["total"] : 0,
  };
}

function llmPath(host: string, id: string): string {
  return `/ui/api/admin/llm/seathosts/${encodeURIComponent(host)}/profiles/${encodeURIComponent(id)}`;
}

function accessPath(id: string): string {
  return `/ui/api/admin/llm/access/${encodeURIComponent(id)}`;
}

export const api = {
  me: () => json<Me>("GET", "/ui/api/me"),
  login: (name: string, password: string) => json<Me>("POST", "/ui/api/login", { name, password }),
  logout: async () => {
    await request("POST", "/ui/api/logout");
  },
  facets: async (filter: Filter = {}) =>
    facetsOf(await json<unknown>("GET", withFilter("/ui/api/facets", scopeOf(filter)))),
  stats: () => json<ReportStats>("GET", "/ui/api/stats"),
  health: () => json<Health>("GET", "/health"),
  reports: (filter: Filter) => {
    const query = filterQuery({ ...filter, limit: undefined });
    const limit = filter.limit ?? PAGE_SIZE;
    return json<Listing>("GET", `/ui/api/reports?limit=${limit}${query ? `&${query}` : ""}`);
  },
  report: (id: string) => json<Report>("GET", reportPath(id)),
  audit: (id: string) => json<AuditEntry[]>("GET", `${reportPath(id)}/audit`),
  change: (id: string, change: Change) => json<Report>("PATCH", reportPath(id), change),
  /** Into the trash; [`api.restore`] takes it out again. */
  trash: async (id: string) => {
    await request("DELETE", reportPath(id));
  },
  restore: async (id: string) => {
    await request("POST", `${reportPath(id)}/restore`);
  },
  /** Gone for good; only from the trash. */
  purge: async (id: string) => {
    await request("POST", `${reportPath(id)}/purge`);
  },
  bulk: (ids: readonly string[], what: { change: BulkChange } | { action: BulkAction }) =>
    json<{ changed: number }>("POST", "/ui/api/reports/bulk", { ids, ...what }),
  addNote: (id: string, text: string) => json<Note[]>("POST", `${reportPath(id)}/notes`, { text }),
  removeNote: async (id: string, note: string) => {
    const response = await request("DELETE", `${reportPath(id)}/notes/${encodeURIComponent(note)}`);
    return (await response.json()) as Note[];
  },
  /** Marks `ids` as opened by this admin, or as not opened. */
  seen: async (ids: readonly string[], seen = true) => {
    await request("POST", "/ui/api/seen", { ids, seen });
  },
  crashes: (filter: Filter) => json<CrashGroup[]>("GET", withFilter("/ui/api/crashes", scopeOf(filter))),
  neighbours: (id: string, filter: Filter) =>
    json<Neighbours>("GET", withFilter(`${reportPath(id)}/neighbours`, { ...filter, limit: undefined, offset: undefined })),
  // The saved views and the admins only add to the list: an answer that is
  // no list is none, never a page that stops drawing.
  views: async () => listOf<SavedView>(await json<unknown>("GET", "/ui/api/views")),
  saveView: (name: string, query: string) => json<SavedView[]>("POST", "/ui/api/views", { name, query }),
  removeView: async (id: string) => {
    const response = await request("DELETE", `/ui/api/views/${encodeURIComponent(id)}`);
    return (await response.json()) as SavedView[];
  },
  admins: async () =>
    listOf<unknown>(await json<unknown>("GET", "/ui/api/admins")).filter((a): a is string => typeof a === "string"),
  /** The filter's reports as CSV, at most 5000, made by the service. */
  csvUrl: (filter: Filter) => withFilter("/ui/api/reports.csv", { ...filter, limit: undefined, offset: undefined }),
  recordUrl: (id: string) => `${reportPath(id)}/record`,
  contact: {
    list: (status?: ContactStatus) =>
      json<ContactListing>("GET", status === undefined ? "/ui/api/contact" : `/ui/api/contact?status=${status}`),
    one: (id: string) => json<ContactDetail>("GET", `/ui/api/contact/${encodeURIComponent(id)}`),
    mark: (id: string, status: ContactStatus) =>
      json<ContactDetail>("PATCH", `/ui/api/contact/${encodeURIComponent(id)}`, { status }),
    remove: async (id: string) => {
      await request("DELETE", `/ui/api/contact/${encodeURIComponent(id)}`);
    },
  },
  admin: {
    stats: () => json<Stats>("GET", "/ui/api/admin/stats"),
    live: () => json<Live>("GET", "/ui/api/admin/live"),
    offline: () => json<Offline>("GET", "/ui/api/admin/offline"),
    metrics: () => json<Metrics>("GET", "/ui/api/admin/metrics"),
    sets: async () => {
      const overview = await json<SetsOverview>("GET", "/ui/api/admin/sets");
      return {
        ...overview,
        pool: withGenerated(overview.pool),
        sets: overview.sets.map(withGenerated),
      };
    },
    set: async (code: string) =>
      withGenerated(await json<SetDetail>("GET", `/ui/api/admin/sets/${encodeURIComponent(code)}`)),
    accounts: (filter: AccountFilter) => {
      const query = accountQuery(filter);
      return json<AccountPage>(
        "GET",
        `/ui/api/admin/accounts?limit=${ACCOUNT_PAGE}${query ? `&${query}` : ""}`,
      );
    },
    account: (id: string) => json<AccountDetail>("GET", `/ui/api/admin/accounts/${encodeURIComponent(id)}`),
    invites: () => json<Invite[]>("GET", "/ui/api/admin/invites"),
    create: (order: InviteOrder) => json<MadeKeys>("POST", "/ui/api/admin/invites", order),
    revoke: async (id: string) => {
      await request("DELETE", `/ui/api/admin/invites/${encodeURIComponent(id)}`);
    },
    audit: () => json<ConsoleChange[]>("GET", "/ui/api/admin/audit"),
    central: {
      link: () => json<CentralLink>("GET", "/ui/api/admin/central"),
      paste: (token: string) => json<CentralLink>("PUT", "/ui/api/admin/central/token", { token }),
      remove: async () => {
        await request("DELETE", "/ui/api/admin/central/token");
      },
    },
    cards: {
      search: async (q: string) =>
        (await json<{ cards: CardHit[] }>("GET", `/ui/api/admin/cards/search?q=${encodeURIComponent(q)}`)).cards,
      blocks: () => json<CardBlock[]>("GET", "/ui/api/admin/cards/blocks"),
      block: (order: BlockOrder) => json<CardBlock>("POST", "/ui/api/admin/cards/blocks", order),
      unblock: async (id: string) => {
        await request("DELETE", `/ui/api/admin/cards/blocks/${encodeURIComponent(id)}`);
      },
    },
    llm: {
      seathosts: async () => (await json<{ seathosts: SeatHost[] }>("GET", "/ui/api/admin/llm/seathosts")).seathosts,
      write: async (host: string, id: string, definition: LlmDefinition) => {
        await request("PUT", llmPath(host, id), definition);
      },
      remove: async (host: string, id: string) => {
        await request("DELETE", llmPath(host, id));
      },
      enable: async (host: string, id: string, enabled: boolean) => {
        await request("POST", `${llmPath(host, id)}/enabled`, { enabled });
      },
      probe: async (host: string, id: string) => {
        await request("POST", `${llmPath(host, id)}/probe`);
      },
      /** Write-only: `null` forgets the key; no route ever answers one. */
      key: async (host: string, id: string, key: string | null) => {
        await request("POST", `${llmPath(host, id)}/key`, { key });
      },
      access: async () => (await json<{ profiles: LlmAccess[] }>("GET", "/ui/api/admin/llm/access")).profiles,
      accessOf: (id: string) => json<LlmAccessDetail>("GET", accessPath(id)),
      setPolicy: async (id: string, policy: LlmPolicy, defaultForRegistered?: boolean) => {
        const body: { policy: LlmPolicy; default_for_registered?: boolean } = { policy };
        if (defaultForRegistered !== undefined) body.default_for_registered = defaultForRegistered;
        await request("PUT", accessPath(id), body);
      },
      allow: (id: string, accounts: string[]) =>
        json<LlmAccessDetail>("POST", `${accessPath(id)}/allow`, { accounts }),
      disallow: (id: string, accounts: string[]) =>
        json<LlmAccessDetail>("POST", `${accessPath(id)}/disallow`, { accounts }),
    },
    /** Registered accounts whose username or display name starts with `q`; at most ten. */
    lookup: async (q: string) =>
      (await json<{ accounts: AccountCandidate[] }>("GET", `/ui/api/admin/accounts/lookup?q=${encodeURIComponent(q)}`))
        .accounts,
  },
};
