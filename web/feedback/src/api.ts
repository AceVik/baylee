// The service's `/ui/api` routes (docs/feedback.md §"The web UI"). Every
// request is same-origin and carries the session cookie; every change also
// carries `X-Baylee-CSRF: 1`, which a form on another site cannot send.

export const KINDS = ["bug", "improvement", "feedback", "crash", "other"] as const;
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
}

export interface Report extends Summary {
  client: Record<string, unknown>;
}

export interface Listing {
  total: number;
  reports: Summary[];
}

export interface Count {
  value: string;
  count: number;
}

export interface Facets {
  gateways: Count[];
  statuses: Count[];
  kinds: Count[];
  reporters: Count[];
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

export const COVERAGES = ["implemented", "partial", "unimplemented", "absent"] as const;
export type Coverage = (typeof COVERAGES)[number];

/** One set's progress: the cards first printed in it, by how far each is. */
export interface SetProgress {
  code: string;
  position: number;
  total: number;
  implemented: number;
  partial: number;
  unimplemented: number;
  absent: number;
}

/** `GET /ui/api/admin/sets`. */
export interface SetsOverview {
  at: string;
  version: string;
  corpus: number;
  pool: { cards: number; implemented: number; partial: number; unimplemented: number; absent: number };
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
}

export interface AccountPage {
  total: number;
  online: number;
  accounts: AccountRow[];
}

export interface AccountFilter {
  q?: string;
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
  if (filter.kind && filter.kind !== "all") params.set("kind", filter.kind);
  if (filter.sort && filter.sort !== "newest") params.set("sort", filter.sort);
  if (filter.online) params.set("online", "true");
  if (filter.offset !== undefined && filter.offset > 0) params.set("offset", String(filter.offset));
  return params.toString();
}

/** An account search read back from a query string. */
export function parseAccountFilter(search: string): AccountFilter {
  const params = new URLSearchParams(search);
  const filter: AccountFilter = {};
  const q = params.get("q");
  if (q) filter.q = q;
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

/** A change made through the console, as this service audited it. */
export interface ConsoleChange {
  at: string;
  actor: string;
  action: string;
  detail: string | null;
}

/** The list's filters, as the query string carries them. */
export interface Filter {
  kind?: Kind;
  status?: Status;
  gateway?: string;
  reporter?: string;
  q?: string;
  from?: string;
  to?: string;
  has_record?: boolean;
  offset?: number;
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

async function request(
  method: "GET" | "POST" | "PATCH" | "DELETE",
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

async function json<T>(method: "GET" | "POST" | "PATCH", path: string, body?: unknown): Promise<T> {
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
  put("kind", filter.kind);
  put("status", filter.status);
  put("gateway", filter.gateway);
  put("reporter", filter.reporter);
  put("q", filter.q?.trim());
  put("from", filter.from);
  put("to", filter.to);
  put("has_record", filter.has_record);
  if (filter.offset !== undefined && filter.offset > 0) put("offset", filter.offset);
  return params.toString();
}

function oneOf<T extends string>(list: readonly T[], value: string | null): T | undefined {
  return list.find((item) => item === value);
}

const DAY = /^\d{4}-\d{2}-\d{2}$/;

/** A filter read back from a query string; anything malformed is dropped. */
export function parseFilter(search: string): Filter {
  const params = new URLSearchParams(search);
  const filter: Filter = {};
  const kind = oneOf(KINDS, params.get("kind"));
  if (kind) filter.kind = kind;
  const status = oneOf(STATUSES, params.get("status"));
  if (status) filter.status = status;
  for (const key of ["gateway", "reporter", "q"] as const) {
    const value = params.get(key);
    if (value) filter[key] = value;
  }
  for (const key of ["from", "to"] as const) {
    const value = params.get(key);
    if (value && DAY.test(value)) filter[key] = value;
  }
  const record = params.get("has_record");
  if (record === "true" || record === "false") filter.has_record = record === "true";
  const offset = Number(params.get("offset"));
  if (Number.isInteger(offset) && offset > 0) filter.offset = offset;
  return filter;
}

export const api = {
  me: () => json<Me>("GET", "/ui/api/me"),
  login: (name: string, password: string) => json<Me>("POST", "/ui/api/login", { name, password }),
  logout: async () => {
    await request("POST", "/ui/api/logout");
  },
  facets: () => json<Facets>("GET", "/ui/api/facets"),
  stats: () => json<ReportStats>("GET", "/ui/api/stats"),
  health: () => json<Health>("GET", "/health"),
  reports: (filter: Filter) => {
    const query = filterQuery(filter);
    return json<Listing>("GET", `/ui/api/reports?limit=${PAGE_SIZE}${query ? `&${query}` : ""}`);
  },
  report: (id: string) => json<Report>("GET", `/ui/api/reports/${encodeURIComponent(id)}`),
  audit: (id: string) => json<AuditEntry[]>("GET", `/ui/api/reports/${encodeURIComponent(id)}/audit`),
  change: (id: string, change: { status?: Status; issue?: number | null }) =>
    json<Report>("PATCH", `/ui/api/reports/${encodeURIComponent(id)}`, change),
  remove: async (id: string) => {
    await request("DELETE", `/ui/api/reports/${encodeURIComponent(id)}`);
  },
  recordUrl: (id: string) => `/ui/api/reports/${encodeURIComponent(id)}/record`,
  admin: {
    stats: () => json<Stats>("GET", "/ui/api/admin/stats"),
    live: () => json<Live>("GET", "/ui/api/admin/live"),
    offline: () => json<Offline>("GET", "/ui/api/admin/offline"),
    metrics: () => json<Metrics>("GET", "/ui/api/admin/metrics"),
    sets: () => json<SetsOverview>("GET", "/ui/api/admin/sets"),
    set: (code: string) => json<SetDetail>("GET", `/ui/api/admin/sets/${encodeURIComponent(code)}`),
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
  },
};
