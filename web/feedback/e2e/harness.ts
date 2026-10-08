// A real baylee-feedback for the end-to-end tests: a schema of its own in
// the PostgreSQL that DATABASE_URL names, an admin made through the binary's
// own CLI, the built UI from `dist/`, and reports handed in through the
// intake route a gateway uses.

import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { crc32, deflateSync } from "node:zlib";

import postgres from "postgres";

export const ADMIN = "e2e-admin";
export const PASSWORD = "an e2e password, long enough";
export const GATEWAY_TOKEN = "e2e-gateway-token-000000000001";
export const PORT = Number(process.env["E2E_PORT"] ?? "28791");
export const BASE_URL = `http://localhost:${PORT}`;
/** The gateway's `BAYLEE_ADMIN_TOKEN`, which the service is given too. */
export const CONSOLE_TOKEN = "e2e-console-token-0123456789abcdef0123456789";

const here = import.meta.dirname;
const repo = resolve(here, "../../..");
const dist = resolve(here, "../dist");

function binary(name = "baylee-feedback", variable = "BAYLEE_FEEDBACK_BIN"): string {
  const path = process.env[variable] ?? resolve(repo, `target/debug/${name}`);
  if (!existsSync(path)) {
    throw new Error(`${path} is not built: cargo build -p ${name}`);
  }
  return path;
}

/** The gateway the admin console talks to: its public port and its console's. */
export interface GatewayPorts {
  port: number;
  admin: number;
}

let gatewayPorts: GatewayPorts | null = null;

/** Where the e2e gateway listens; set by [`start`], read by the specs via a file. */
export function gateway(): GatewayPorts {
  if (gatewayPorts !== null) return gatewayPorts;
  const file = resolve(tmpdir(), `baylee-feedback-e2e-gateway-${PORT}.json`);
  gatewayPorts = JSON.parse(readFileSync(file, "utf8")) as GatewayPorts;
  return gatewayPorts;
}

async function readPort(file: string, child: ChildProcess, what: string): Promise<number> {
  for (let i = 0; i < 300; i += 1) {
    if (child.exitCode !== null) throw new Error(`${what} exited with ${child.exitCode}`);
    if (existsSync(file)) {
      const port = Number(readFileSync(file, "utf8").trim());
      if (port > 0) return port;
    }
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`${what} never wrote ${file}`);
}

/** Registers `username` on the gateway with `key`; the status. */
export async function register(username: string, key: string): Promise<number> {
  const response = await fetch(`http://127.0.0.1:${gateway().port}/auth/register`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      username,
      display_name: username,
      password: "a-very-fine-password",
      invite_key: key,
    }),
  });
  return response.status;
}

/**
 * A real baylee-gateway, a closed beta with its admin console on a loopback
 * port of its own, in a schema of its own; two accounts let in with keys
 * its own command made.
 */
async function startGateway(url: string, sql: postgres.Sql): Promise<{ child: ChildProcess; schema: string }> {
  const bin = binary("baylee-gateway", "BAYLEE_GATEWAY_BIN");
  const schema = `gwe2e_${Date.now()}_${process.pid}`;
  await sql.unsafe(`CREATE SCHEMA "${schema}"`);
  const scoped = `${url}${url.includes("?") ? "&" : "?"}options=-c%20search_path%3D${schema},public`;
  const work = mkdtempSync(resolve(tmpdir(), "baylee-gateway-e2e-"));
  const portFile = resolve(work, "port");
  const adminFile = resolve(work, "admin-port");
  const child = spawn(bin, [], {
    cwd: work,
    env: {
      ...process.env,
      DATABASE_URL: scoped,
      PORT: "0",
      BAYLEE_PORT_FILE: portFile,
      BAYLEE_ADMIN_TOKEN: CONSOLE_TOKEN,
      BAYLEE_ADMIN_BIND: "127.0.0.1:0",
      BAYLEE_ADMIN_PORT_FILE: adminFile,
      BAYLEE_REGISTRATION: "invite",
      BAYLEE_DB_POOL: "2",
      BAYLEE_ART_PATH: "off",
      BAYLEE_DECK_IMAGE_PATH: "off",
      STORE_PATH: resolve(work, "store.json"),
      RUST_LOG: "warn",
    },
    stdio: ["ignore", "inherit", "inherit"],
  });
  const port = await readPort(portFile, child, "baylee-gateway");
  const admin = await readPort(adminFile, child, "baylee-gateway's console");
  gatewayPorts = { port, admin };
  writeFileSync(resolve(tmpdir(), `baylee-feedback-e2e-gateway-${PORT}.json`), JSON.stringify(gatewayPorts));

  const keys = execFileSync(bin, ["invite", "create", "--count", "2", "--note", "seed"], {
    env: { ...process.env, DATABASE_URL: scoped, RUST_LOG: "off" },
  })
    .toString()
    .trim()
    .split("\n");
  for (const [n, key] of keys.entries()) {
    const status = await register(`player${n}`, key);
    if (status !== 200) throw new Error(`registering a seeded player answered ${status}`);
  }
  return { child, schema };
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

  const gw = await startGateway(url, sql);
  const child = spawn(bin, [], {
    env: {
      ...env,
      FEEDBACK_BIND: `127.0.0.1:${PORT}`,
      FEEDBACK_WEB_DIR: dist,
      FEEDBACK_GATEWAY_TOKENS: `eu=${GATEWAY_TOKEN}`,
      FEEDBACK_GATEWAY_ADMIN_URL: `http://127.0.0.1:${gateway().admin}`,
      FEEDBACK_GATEWAY_ADMIN_TOKEN: CONSOLE_TOKEN,
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
      gw.child.kill("SIGTERM");
      await new Promise((r) => setTimeout(r, 300));
      await sql.unsafe(`DROP SCHEMA "${schema}" CASCADE`);
      await sql.unsafe(`DROP SCHEMA "${gw.schema}" CASCADE`);
      await sql.end();
    },
  };
}
