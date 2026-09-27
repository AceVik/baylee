// A stand-in for the service in component tests: `fetch` answers from a
// table of routes, and every call is kept for the test to read.

import { vi } from "vitest";

import type { Report, Summary } from "./api";

export interface Call {
  method: string;
  url: string;
  headers: Record<string, string>;
  body: unknown;
}

type Handler = (call: Call) => { status?: number; body?: unknown } | undefined;

/** Replaces `fetch`; `handler` answers each call, unanswered ones get 404. */
export function serve(handler: Handler): Call[] {
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: string, init?: RequestInit) => {
      const headers: Record<string, string> = {};
      for (const [k, v] of Object.entries((init?.headers ?? {}) as Record<string, string>)) {
        headers[k.toLowerCase()] = v;
      }
      const call: Call = {
        method: init?.method ?? "GET",
        url: input,
        headers,
        body: typeof init?.body === "string" ? JSON.parse(init.body) : undefined,
      };
      calls.push(call);
      const answer = handler(call) ?? { status: 404, body: { error: "no such route" } };
      const status = answer.status ?? 200;
      return new Response(status === 204 ? null : JSON.stringify(answer.body ?? {}), {
        status,
        headers: { "Content-Type": "application/json" },
      });
    }),
  );
  return calls;
}

export function summary(over: Partial<Summary> = {}): Summary {
  return {
    id: "0199aaaa-0000-7000-8000-000000000001",
    created_at: "2026-09-27T12:00:00Z",
    updated_at: "2026-09-27T12:00:00Z",
    gateway: "eu",
    gateway_name: "Baylee EU",
    gateway_url: "https://eu.example",
    gateway_version: "0.1.0-beta.1+build.42 (3f9a1c7e21)",
    reporter: "5bdc0e1f9a7c33aa",
    kind: "bug",
    status: "new",
    text: "The Swamp untapped by itself.",
    game_id: "g1",
    has_record: false,
    record_complete: null,
    record_bytes: 0,
    issue_number: null,
    issue_url: null,
    ...over,
  };
}

export function fullReport(over: Partial<Report> = {}): Report {
  return {
    ...summary({ has_record: true, record_complete: true, record_bytes: 21_606 }),
    client: {
      build: { version: "0.1.0-beta.1", commit: "3f9a1c7e21" },
      system: {
        platform: "macos/aarch64",
        cpus: 10,
        adapter: "Apple M1 Max",
        backend: "Metal",
        window: [1280, 800],
        scale: 2,
        lang: "en",
      },
      log: {
        seat: 0,
        roster: [
          { seat: 0, name: "You", is_ai: false, team: null },
          { seat: 1, name: "Player A", is_ai: true, team: 2 },
        ],
        lines: [
          { turn: 1, at: 1_790_000_000_000, text: "You played Swamp." },
          { turn: 2, at: 0, text: "The Swamp untapped on its own." },
        ],
      },
      settings: { lang: "en", music: "on" },
      game: { table: { seat: 0, seq: 42, when: "upkeep" }, view: { turn: 2 } },
      screenshot: { width: 2, height: 1, png_base64: "iVBORw0KGgo=" },
      crash: {
        message: "index out of bounds",
        location: "src/lobby.rs:1:2",
        backtrace: "0: main",
        at_unix: 1_790_000_000,
        platform: "linux/x86_64",
        thread: "main",
      },
    },
    ...over,
  };
}
