// A JSON value as a tree: objects and arrays fold (native <details>, so the
// keyboard and screen readers know them), long strings are cut until asked.

import { useState } from "react";

import { isRecord } from "./api";

/** Strings longer than this show their start and a button for the rest. */
export const LONG_STRING = 240;

/** Keys whose value is not worth reading as text. */
const OPAQUE = new Set(["png_base64", "gzip_base64"]);

function LongString({ text }: { text: string }) {
  const [all, setAll] = useState(false);
  if (text.length <= LONG_STRING || all) return <span className="j-string">"{text}"</span>;
  return (
    <>
      <span className="j-string">"{text.slice(0, LONG_STRING)}…"</span>{" "}
      <button
        type="button"
        className="link"
        onClick={() => {
          setAll(true);
        }}
      >
        all {text.length} characters
      </button>
    </>
  );
}

function Leaf({ value, name }: { value: unknown; name: string | null }) {
  if (typeof value === "string" && name !== null && OPAQUE.has(name))
    return <span className="muted">({value.length} characters of base64)</span>;
  if (typeof value === "string") return <LongString text={value} />;
  if (typeof value === "number") return <span className="j-number">{String(value)}</span>;
  if (typeof value === "boolean") return <span className="j-bool">{String(value)}</span>;
  if (value === null) return <span className="j-null">null</span>;
  return <span className="muted">{typeof value}</span>;
}

function Node({ name, value, depth, open }: { name: string | null; value: unknown; depth: number; open: number }) {
  const label = name === null ? null : <span className="j-key">{name}: </span>;
  const entries: [string, unknown][] | null = Array.isArray(value)
    ? value.map((v, i) => [String(i), v])
    : isRecord(value)
      ? Object.entries(value)
      : null;
  if (entries === null) {
    return (
      <li>
        {label}
        <Leaf value={value} name={name} />
      </li>
    );
  }
  const brackets = Array.isArray(value) ? ["[", "]"] : ["{", "}"];
  if (entries.length === 0) {
    return (
      <li>
        {label}
        <span className="muted">
          {brackets[0]}
          {brackets[1]}
        </span>
      </li>
    );
  }
  return (
    <li>
      <details open={depth < open}>
        <summary>
          {label}
          <span className="muted">
            {brackets[0]} {entries.length} {Array.isArray(value) ? "items" : "keys"} {brackets[1]}
          </span>
        </summary>
        <ul className="j-children">
          {entries.map(([k, v]) => (
            <Node key={k} name={k} value={v} depth={depth + 1} open={open} />
          ))}
        </ul>
      </details>
    </li>
  );
}

/** `value` as a tree, the first `open` levels unfolded. */
export function JsonTree({ value, open = 1 }: { value: unknown; open?: number }) {
  return (
    <ul className="json-tree">
      <Node name={null} value={value} depth={0} open={open} />
    </ul>
  );
}
