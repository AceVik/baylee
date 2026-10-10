import { describe, expect, test } from "vitest";

import { alias } from "./alias";
import { field, toCsv } from "./csv";
import { pendingLabel, pendingName, readGame } from "./game";
import { cardUrl, imageUrl, isScryfallId, setUrl } from "./scryfall";
import { forgetSeen, isSeen, markAllSeen, markSeen, SEEN_LIMIT } from "./seen";
import { summary } from "../test-utils";

describe("a reporter's alias", () => {
  test("is the same every time, two words and the pseudonym's start", () => {
    const a = alias("5bdc0e1f9a7c33aa");
    expect(a).toEqual(alias("5bdc0e1f9a7c33aa"));
    expect(a.name).toMatch(/^[A-Z][a-z]+ [A-Z][a-z]+$/);
    expect(a.short).toBe("5bdc");
    expect(a.hue).toBeGreaterThanOrEqual(0);
    expect(a.hue).toBeLessThan(360);
  });

  test("tells two pseudonyms apart, and reads a pseudonym that is not hex", () => {
    expect(alias("5bdc0e1f9a7c33aa").name).not.toBe(alias("5bdc0e1e9a7c33aa").name);
    const odd = alias("not hex at all");
    expect(odd.name).toMatch(/^[A-Z][a-z]+ [A-Z][a-z]+$/);
    expect(odd.short).toBe("not ");
  });
});

describe("the CSV export", () => {
  test("quotes what needs quoting", () => {
    expect(field("plain")).toBe("plain");
    expect(field('say "hi", twice')).toBe('"say ""hi"", twice"');
    expect(field("two\nlines")).toBe('"two\nlines"');
    expect(field(null)).toBe("");
    expect(field(12)).toBe("12");
  });

  test("writes a header and one line per report", () => {
    const csv = toCsv([
      summary({ text: "The Swamp, untapped" }),
      summary({ id: "two", channel: "direct", gateway: "(direct)", issue_number: 311 }),
    ]);
    const lines = csv.split("\r\n");
    expect(lines[0]).toBe(
      "id,created_at,updated_at,kind,status,gateway,channel,reporter,game_id,has_record,record_bytes,issue_number,text",
    );
    expect(lines[1]).toContain('"The Swamp, untapped"');
    expect(lines[2]).toContain(",(direct),direct,");
    expect(lines[2]).toContain(",311,");
    expect(lines[3]).toBe("");
  });
});

describe("the table a report was written at", () => {
  test("reads the question's name from serde's externally tagged shape", () => {
    expect(pendingName("PassPriority")).toBe("PassPriority");
    expect(pendingName({ ChooseAttackers: { player: 1 } })).toBe("ChooseAttackers");
    expect(pendingName({ a: 1, b: 2 })).toBeNull();
    expect(pendingName(null)).toBeNull();
    expect(pendingLabel("ChooseManaAbility")).toBe("Choose mana ability");
  });

  test("reads turn, seat, question and what the client held", () => {
    const game = readGame({
      game: {
        table: { seat: 1, seq: 42, when: "T7 main 1" },
        view: { turn: 7, active: 0, phase: "Main1", step: "Main", awaiting: 1, players: [{}, {}] },
        pending: { ChooseAttackers: { player: 1, options: [1, 2], note: "x".repeat(50) } },
        holding: { selected: 2, armed: "Play Swamp", mana_run: true, outbox: 1, last_error: null },
      },
    });
    expect(game).toEqual({
      seat: 1,
      seq: 42,
      when: "T7 main 1",
      turn: 7,
      phase: "Main1",
      step: "Main",
      activeSeat: 0,
      awaiting: 1,
      pending: "ChooseAttackers",
      pendingDetail: "player: 1 · options: 2",
      holding: { selected: 2, armed: "Play Swamp", manaRun: true, outbox: 1, lastError: null },
      players: 2,
    });
  });

  test("none, or a malformed part, reads as none", () => {
    expect(readGame({})).toBeNull();
    expect(readGame({ game: "x" })).toBeNull();
    const bare = readGame({ game: { table: "no", pending: 7 } });
    expect(bare?.seat).toBeNull();
    expect(bare?.pending).toBeNull();
    expect(bare?.holding).toBeNull();
  });
});

describe("Scryfall addresses", () => {
  const id = "e3285e6b-3e79-4d7c-bf96-d920f973b80b";

  test("are built from an id shaped like one, on Scryfall's own hosts", () => {
    expect(isScryfallId(id)).toBe(true);
    expect(imageUrl(id)).toBe(`https://cards.scryfall.io/normal/front/e/3/${id}.jpg`);
    expect(imageUrl(id, "back", "small")).toBe(`https://cards.scryfall.io/small/back/e/3/${id}.jpg`);
    expect(cardUrl(id)).toBe(`https://scryfall.com/card/${id}`);
    expect(setUrl("LEA")).toBe("https://scryfall.com/sets/lea");
  });

  test("refuse anything else, so nothing a report says reaches a URL", () => {
    for (const bad of ["", "x", "../x", `${id}/x`, "E3285E6B-3E79-4D7C-BF96-D920F973B80B"]) {
      expect(imageUrl(bad)).toBeNull();
      expect(cardUrl(bad)).toBeNull();
    }
    expect(setUrl("a/b")).toBeNull();
    expect(setUrl("")).toBeNull();
  });
});

describe("what was opened on this device", () => {
  test("is remembered, bounded, and forgotten on request", () => {
    forgetSeen();
    expect(isSeen("a")).toBe(false);
    markSeen("a");
    expect(isSeen("a")).toBe(true);
    markAllSeen(["b", "c"]);
    expect(isSeen("c")).toBe(true);
    markAllSeen(Array.from({ length: SEEN_LIMIT + 10 }, (_, i) => `n${i}`));
    expect(isSeen("a")).toBe(false);
    expect(isSeen(`n${SEEN_LIMIT + 9}`)).toBe(true);
    forgetSeen();
    expect(isSeen("c")).toBe(false);
  });
});
