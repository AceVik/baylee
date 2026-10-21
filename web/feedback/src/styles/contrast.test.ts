// The tokens hold WCAG 2.1 AA in both schemes: text 4.5:1 against every
// surface it is drawn on (1.4.3), and a control's edge, a chart's mark and
// the focus ring 3:1 (1.4.11). Measured from `tokens.css` itself, so a
// colour changed there is measured again.

import { describe, expect, test } from "vitest";

import source from "./tokens.css?raw";

type Scheme = Map<string, string>;

/** The custom properties of the first block after `marker`. */
function block(marker: string): Scheme {
  const start = source.indexOf(marker);
  if (start < 0) throw new Error(`no ${marker} in tokens.css`);
  const open = source.indexOf(":root {", start);
  const close = source.indexOf("}", open);
  const tokens = new Map<string, string>();
  for (const [, name, value] of source.slice(open, close).matchAll(/--([a-z0-9-]+):\s*([^;]+);/g)) {
    if (name !== undefined && value !== undefined) tokens.set(name, value.trim());
  }
  return tokens;
}

const light = block("\n:root {");
const dark = new Map([...light, ...block("@media (prefers-color-scheme: dark)")]);

function rgb(scheme: Scheme, name: string): [number, number, number] {
  const value = scheme.get(name);
  const hex = /^#([0-9a-f]{6})$/i.exec(value ?? "")?.[1];
  if (hex === undefined) throw new Error(`--${name} is not a #rrggbb colour: ${value}`);
  return [0, 2, 4].map((i) => parseInt(hex.slice(i, i + 2), 16)) as [number, number, number];
}

/** One channel of sRGB, linear. */
function lin(c: number): number {
  const s = c / 255;
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}

function luminance([r, g, b]: [number, number, number]): number {
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function ratio(scheme: Scheme, fg: string, bg: string): number {
  const a = luminance(rgb(scheme, fg));
  const b = luminance(rgb(scheme, bg));
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

const SURFACES = ["bg", "surface", "surface-alt", "surface-hover"];

/** Text and what it is drawn on. */
const TEXT: [string, string[]][] = [
  ["text", [...SURFACES, "accent-soft", "accent-soft-strong", "neutral-soft", "warn-soft", "danger-soft", "ok-soft", "info-soft"]],
  ["text-muted", [...SURFACES, "accent-soft", "neutral-soft"]],
  ["text-subtle", ["bg", "surface", "surface-hover"]],
  ["accent", [...SURFACES, "accent-soft"]],
  ["on-accent", ["accent", "accent-hover"]],
  ["on-danger", ["danger"]],
  ["on-ok", ["ok"]],
  ["ok", ["surface", "ok-soft"]],
  ["warn", ["surface", "warn-soft"]],
  ["danger", ["surface", "danger-soft"]],
  ["info", ["surface", "info-soft"]],
  ["bug", ["surface", "neutral-soft"]],
  ["crash", ["surface", "neutral-soft"]],
  ["improvement", ["surface", "neutral-soft"]],
  ["feedback", ["surface", "neutral-soft"]],
  ["other", ["surface", "neutral-soft"]],
];

/** Edges, marks and rings, and what they sit on. */
const GRAPHICS: [string, string[]][] = [
  ["border-strong", SURFACES],
  ["focus", SURFACES],
  ["accent", SURFACES],
  ["c1", ["surface"]],
  ["c2", ["surface"]],
  ["ok", ["surface"]],
];

for (const [name, scheme] of [
  ["light", light],
  ["dark", dark],
] as const) {
  describe(`the ${name} tokens`, () => {
    test("text holds 4.5:1 on every surface it is drawn on", () => {
      const low = TEXT.flatMap(([fg, bgs]) =>
        bgs.map((bg) => ({ pair: `--${fg} on --${bg}`, ratio: ratio(scheme, fg, bg) })),
      ).filter((p) => p.ratio < 4.5);
      expect(low.map((p) => `${p.pair}: ${p.ratio.toFixed(2)}`)).toEqual([]);
    });

    test("edges, marks and the focus ring hold 3:1", () => {
      const low = GRAPHICS.flatMap(([fg, bgs]) =>
        bgs.map((bg) => ({ pair: `--${fg} on --${bg}`, ratio: ratio(scheme, fg, bg) })),
      ).filter((p) => p.ratio < 3);
      expect(low.map((p) => `${p.pair}: ${p.ratio.toFixed(2)}`)).toEqual([]);
    });
  });
}
