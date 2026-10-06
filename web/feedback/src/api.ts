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
};
