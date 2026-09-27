import { describe, expect, test } from "vitest";

import { readBuild, readCrash, readLog, readScreenshot, readSystem } from "./dump";
import { formatBytes, formatClock, formatTime, isBase64 } from "./format";
import { fullReport } from "./test-utils";

describe("each part of a full dump", () => {
  const client = fullReport().client;

  test("build and system", () => {
    expect(readBuild(client)).toEqual({ version: "0.1.0-beta.1", commit: "3f9a1c7e21" });
    expect(readSystem(client)).toEqual([
      ["Platform", "macos/aarch64"],
      ["CPUs", "10"],
      ["Graphics adapter", "Apple M1 Max"],
      ["Graphics backend", "Metal"],
      ["Window", "1280 × 800"],
      ["Scale", "2"],
      ["Language", "en"],
    ]);
  });

  test("screenshot, as a data URL of PNG", () => {
    expect(readScreenshot(client)).toEqual({
      width: 2,
      height: 1,
      src: "data:image/png;base64,iVBORw0KGgo=",
    });
  });

  test("log, with the roster and an undated line", () => {
    const log = readLog(client);
    expect(log?.roster).toEqual([
      { seat: 0, name: "You", isAi: false, team: null },
      { seat: 1, name: "Player A", isAi: true, team: 2 },
    ]);
    expect(log?.lines).toEqual([
      { turn: 1, clock: "14:13:20", text: "You played Swamp." },
      { turn: 2, clock: "", text: "The Swamp untapped on its own." },
    ]);
  });

  test("crash", () => {
    expect(readCrash(client)).toEqual({
      message: "index out of bounds",
      location: "src/lobby.rs:1:2",
      backtrace: "0: main",
      at: "2026-09-21T14:13:20.000Z",
      platform: "linux/x86_64",
      thread: "main",
    });
  });
});

describe("a missing or malformed part reads as nothing", () => {
  test("an empty client", () => {
    for (const read of [readBuild, readSystem, readScreenshot, readLog, readCrash]) {
      expect(read({})).toBeNull();
    }
  });

  test("parts of the wrong shape", () => {
    const client = {
      build: "0.1.0",
      system: [1, 2],
      screenshot: { png_base64: "not base64!\"><script>" },
      log: { lines: "nope", roster: [null, 3, { name: 7 }] },
      crash: { at_unix: -1 },
    };
    expect(readBuild(client)).toBeNull();
    expect(readSystem(client)).toBeNull();
    expect(readScreenshot(client)).toBeNull();
    expect(readLog(client)).toEqual({
      seat: null,
      roster: [{ seat: null, name: "?", isAi: false, team: null }],
      lines: [],
    });
    expect(readCrash(client)).toEqual({
      message: "(no message)",
      location: null,
      backtrace: null,
      at: null,
      platform: null,
      thread: null,
    });
    expect(readSystem({ system: { window: [1], cpus: "ten" } })).toEqual([]);
  });

  test("a screenshot's data never leaves the base64 alphabet", () => {
    expect(readScreenshot({ screenshot: { png_base64: "AAAA,javascript:alert(1)" } })).toBeNull();
    expect(readScreenshot({ screenshot: { png_base64: "" } })).toBeNull();
    expect(readScreenshot({ screenshot: { png_base64: "AAA" } })).toBeNull();
  });
});

describe("format", () => {
  test.each([
    [0, "0 B"],
    [1023, "1023 B"],
    [1024, "1.0 KiB"],
    [21_606, "21 KiB"],
    [3.4 * 1024 * 1024, "3.4 MiB"],
  ])("%d bytes read %s", (bytes, text) => {
    expect(formatBytes(bytes)).toBe(text);
  });

  test("times", () => {
    expect(formatTime("2026-09-27T12:34:56Z")).toBe("2026-09-27 12:34 UTC");
    expect(formatTime("soon")).toBe("soon");
    expect(formatClock(0)).toBe("");
    expect(formatClock(Number.NaN)).toBe("");
  });

  test("base64", () => {
    expect(isBase64("iVBORw0KGgo=")).toBe(true);
    expect(isBase64("iVBO")).toBe(true);
    expect(isBase64("iVB")).toBe(false);
    expect(isBase64("iV O")).toBe(false);
  });
});
