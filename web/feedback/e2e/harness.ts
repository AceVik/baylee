// A real baylee-feedback for the end-to-end tests: a schema of its own in
// the PostgreSQL that DATABASE_URL names, an admin made through the binary's
// own CLI, the built UI from `dist/`, and reports handed in through the
// intake route a gateway uses.

import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { crc32, deflateSync } from "node:zlib";

import postgres from "postgres";

export const ADMIN = "e2e-admin";
export const PASSWORD = "an e2e password, long enough";
export const GATEWAY_TOKEN = "e2e-gateway-token-000000000001";
export const PORT = Number(process.env["E2E_PORT"] ?? "28791");
export const BASE_URL = `http://localhost:${PORT}`;

const here = import.meta.dirname;
const repo = resolve(here, "../../..");
const dist = resolve(here, "../dist");

function binary(): string {
  const path = process.env["BAYLEE_FEEDBACK_BIN"] ?? resolve(repo, "target/debug/baylee-feedback");
  if (!existsSync(path)) {
    throw new Error(`${path} is not built: cargo build -p baylee-feedback`);
  }
  return path;
}

function databaseUrl(): string {
  const url = process.env["DATABASE_URL"];
  if (!url) {
    throw new Error(
      "DATABASE_URL is not set, and these tests need PostgreSQL:\n" +
        "  docker compose up -d\n  export DATABASE_URL=postgres://baylee:baylee@127.0.0.1:5432/baylee",
    );
  }
  return url;
}

/** One PNG chunk: length, type, data, CRC. */
function chunk(type: string, data: Buffer): Buffer {
  const head = Buffer.alloc(4);
  head.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([head, body, crc]);
}

/** A PNG of `width` × `height`, a calm gradient with a band, in base64: a stand-in screenshot. */
export function screenshotPng(width: number, height: number): string {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 2; // RGB
  const rows = Buffer.alloc((width * 3 + 1) * height);
  for (let y = 0; y < height; y += 1) {
    const row = y * (width * 3 + 1);
    for (let x = 0; x < width; x += 1) {
      const band = Math.abs(y - height * 0.6) < height * 0.08;
      const at = row + 1 + x * 3;
      rows[at] = band ? 200 : 30 + Math.round((x / width) * 40);
      rows[at + 1] = band ? 170 : 60 + Math.round((y / height) * 60);
      rows[at + 2] = band ? 90 : 90 + Math.round((x / width) * 80);
    }
  }
  const png = Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(rows)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
  return png.toString("base64");
}

export function report(
  kind: string,
  text: string,
  options: { reporter?: string; client?: Record<string, unknown>; record?: boolean } = {},
): Record<string, unknown> {
  return {
    gateway: { name: "Baylee EU", url: "https://eu.example", version: "0.1.0-beta.1+build.42 (3f9a1c7e21)" },
    reporter: options.reporter ?? "5bdc0e1f9a7c33aa",
    kind,
    text,
    game_id: "g-2026-0001",
    client: options.client ?? { build: { version: "0.1.0-beta.1", commit: "3f9a1c7e21" } },
    record: options.record
      ? { complete: true, gzip_base64: Buffer.from("\x1f\x8bnot really a record").toString("base64") }
      : null,
  };
}

/** A report with every part a client can send. */
export function fullClient(): Record<string, unknown> {
  return {
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
        { seat: 1, name: "Player A", is_ai: true, team: null },
      ],
      lines: [
        { turn: 1, at: 1_790_000_000_000, text: "You played Swamp." },
        { turn: 1, at: 1_790_000_005_000, text: "Player A cast Lightning Bolt targeting you." },
        { turn: 2, at: 1_790_000_060_000, text: "The Swamp untapped by itself." },
      ],
    },
    settings: { lang: "en", preview_scale: 1, prefer_text_view: false, zone_view: "fan", music: "on", saved_gateways: 1 },
    game: { table: { seat: 0, seq: 42, when: "turn 2, upkeep" }, view: { turn: 2 }, pending: null, holding: { selected: 0 } },
    screenshot: { width: 640, height: 360, png_base64: screenshotPng(640, 360) },
  };
}

export async function handIn(body: Record<string, unknown>): Promise<string> {
  const response = await fetch(`${BASE_URL}/intake/reports`, {
    method: "POST",
    headers: { Authorization: `Bearer ${GATEWAY_TOKEN}`, "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (response.status !== 201) throw new Error(`intake answered ${response.status}`);
  const made = (await response.json()) as { report_id: string };
  return made.report_id;
}

async function waitForHealth(child: ChildProcess): Promise<void> {
  for (let i = 0; i < 100; i += 1) {
    if (child.exitCode !== null) throw new Error(`baylee-feedback exited with ${child.exitCode}`);
    try {
      const response = await fetch(`${BASE_URL}/health`);
      if (response.ok) return;
    } catch {
      // Not listening yet.
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error("baylee-feedback did not come up");
}

export interface Running {
  stop: () => Promise<void>;
}

/** Starts the service with a fresh schema, an admin and a handful of reports. */
export async function start(): Promise<Running> {
  if (!existsSync(resolve(dist, "index.html"))) throw new Error("dist/ is not built: npm run build");
  const bin = binary();
  const url = databaseUrl();
  const schema = `fbe2e_${Date.now()}_${process.pid}`;
  const sql = postgres(url, { max: 1, onnotice: () => {} });
  await sql.unsafe(`CREATE SCHEMA "${schema}"`);
  const scoped = `${url}${url.includes("?") ? "&" : "?"}options=-c%20search_path%3D${schema}`;
  const env = { ...process.env, FEEDBACK_DATABASE_URL: scoped, RUST_LOG: "warn" };

  // The binary's migrator runs on connect, so `admin add` also makes the tables.
  execFileSync(bin, ["admin", "add", ADMIN], { env, input: `${PASSWORD}\n` });

  const child = spawn(bin, [], {
    env: {
      ...env,
      FEEDBACK_BIND: `127.0.0.1:${PORT}`,
      FEEDBACK_WEB_DIR: dist,
      FEEDBACK_GATEWAY_TOKENS: `eu=${GATEWAY_TOKEN}`,
    },
    stdio: ["ignore", "inherit", "inherit"],
  });
  await waitForHealth(child);

  await handIn(report("bug", "The Swamp untapped by itself during my upkeep.", { client: fullClient(), record: true }));
  await handIn(report("crash", "The client crashed when I opened the deck list.", {
    reporter: "77aa01bc22dd33ee",
    client: {
      build: { version: "0.1.0-beta.1", commit: "3f9a1c7e21" },
      crash: {
        message: "index out of bounds: the len is 3 but the index is 7",
        location: "crates/baylee-client/src/lobby.rs:120:9",
        backtrace: "0: baylee_client::lobby::decks\n1: bevy_ecs::system::run",
        at_unix: 1_790_000_000,
        platform: "linux/x86_64",
        thread: "main",
      },
    },
  }));
  await handIn(report("improvement", "Let me sort the deck list by colour.", { reporter: "77aa01bc22dd33ee" }));
  await handIn(report("feedback", "Lovely music in the lobby.", { client: {} }));

  return {
    stop: async () => {
      child.kill("SIGTERM");
      await new Promise((r) => setTimeout(r, 200));
      await sql.unsafe(`DROP SCHEMA "${schema}" CASCADE`);
      await sql.end();
    },
  };
}
