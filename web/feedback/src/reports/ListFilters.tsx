// The list's filters (`ReportList`): every dimension the service filters
// on, each value with its count under the rest of the filter, and the
// active ones as pills that remove themselves.

import { type ReactNode } from "react";

import {
  KINDS,
  PRIORITIES,
  SORTS,
  STATUSES,
  type Count,
  type Facets,
  type Filter,
  type Sort,
} from "../api";
import { kindLabel, priorityLabel, sortLabel, statusLabel, t, type Lang } from "../i18n";
import { shortId } from "./pieces";

/** A change to the filters; `undefined` clears one. */
export type FilterChange = { [K in keyof Filter]?: Filter[K] | undefined };

/** The sorts read best from the top down; the others newest or most first. */
const ASCENDING_FIRST: ReadonlySet<Sort> = new Set(["status", "kind", "reporter", "gateway"]);

/** The direction a sort starts in when it is picked. */
export function firstDir(sort: Sort): "asc" | "desc" {
  return ASCENDING_FIRST.has(sort) ? "asc" : "desc";
}

/** `values` with `value` added, or taken out when it was there; none left is no filter. */
export function toggled<T extends string>(values: readonly T[] | undefined, value: T, order: readonly T[]): T[] | undefined {
  const now = new Set(values ?? []);
  if (now.has(value)) now.delete(value);
  else now.add(value);
  const next = order.filter((v) => now.has(v));
  return next.length > 0 ? next : undefined;
}

/** A source filter's words: a source, none, or all but either. */
export function sourceLabel(lang: Lang, value: string): string {
  if (value === "none") return t(lang, "rep.noSource");
  if (value.startsWith("!")) return t(lang, "rep.sourceOthers", { source: sourceLabel(lang, value.slice(1)) });
  return value;
}

function countOf(counts: readonly Count[] | undefined, value: string | null): number | undefined {
  return counts?.find((c) => c.value === value)?.count;
}

const field = (id: string, label: string, control: ReactNode) => (
  <div className="field">
    <label htmlFor={id}>{label}</label>
    {control}
  </div>
);

/** A group of toggles, several of which may be on. */
function Chips<T extends string>({
  legend,
  values,
  on,
  label,
  count,
  onToggle,
}: {
  legend: string;
  values: readonly T[];
  on: readonly T[] | undefined;
  label: (value: T) => string;
  count: (value: T) => number | undefined;
  onToggle: (value: T) => void;
}) {
  return (
    <fieldset className="chip-group">
      <legend>{legend}</legend>
      {values.map((value) => {
        const pressed = on?.includes(value) === true;
        const n = count(value);
        return (
          <button
            key={value}
            type="button"
            className={`chip-button${pressed ? " on" : ""}`}
            aria-pressed={pressed}
            onClick={() => {
              onToggle(value);
            }}
          >
            {label(value)}
            {n !== undefined && <span className="seg-count">{n}</span>}
          </button>
        );
      })}
    </fieldset>
  );
}

export function FilterPanel({
  lang,
  filter,
  facets,
  admins,
  me,
  set,
  onSubmit,
  hidden,
}: {
  lang: Lang;
  filter: Filter;
  facets: Facets | null;
  admins: readonly string[];
  me: string;
  set: (change: FilterChange) => void;
  onSubmit: () => void;
  hidden: boolean;
}) {
  const priorities = [...PRIORITIES, "none"] as const;
  const people = [...new Set([me, ...admins])].filter((a) => a !== "");
  const labels = facets?.labels.filter((c): c is Count & { value: string } => c.value !== null) ?? [];
  const sources = facets?.sources.filter((c): c is Count & { value: string } => c.value !== null) ?? [];
  return (
    <form
      id="list-filters"
      className="filters"
      hidden={hidden}
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit();
      }}
      aria-label={t(lang, "rep.filters")}
    >
      <div className="chip-groups">
        <Chips
          legend={t(lang, "rep.status")}
          values={STATUSES}
          on={filter.status}
          label={(s) => statusLabel(lang, s)}
          count={(s) => countOf(facets?.statuses, s)}
          onToggle={(s) => {
            set({ status: toggled(filter.status, s, STATUSES) });
          }}
        />
        <Chips
          legend={t(lang, "rep.kind")}
          values={KINDS}
          on={filter.kind}
          label={(k) => kindLabel(lang, k)}
          count={(k) => countOf(facets?.kinds, k)}
          onToggle={(k) => {
            set({ kind: toggled(filter.kind, k, KINDS) });
          }}
        />
        <Chips
          legend={t(lang, "rep.priority")}
          values={priorities}
          on={filter.priority}
          label={(p) => (p === "none" ? t(lang, "rep.noPriority") : priorityLabel(lang, p))}
          count={(p) => countOf(facets?.priorities, p === "none" ? null : p)}
          onToggle={(p) => {
            set({ priority: toggled(filter.priority, p, priorities) });
          }}
        />
      </div>
      <div className="filter-fields">
        {field(
          "f-assignee",
          t(lang, "rep.assignee"),
          <select
            id="f-assignee"
            value={filter.assignee ?? ""}
            onChange={(event) => {
              set({ assignee: event.target.value || undefined });
            }}
          >
            <option value="">{t(lang, "rep.any")}</option>
            <option value="none">
              {t(lang, "rep.nobody")}
              {countOf(facets?.assignees, null) !== undefined ? ` (${countOf(facets?.assignees, null)})` : ""}
            </option>
            {people.map((name) => (
              <option key={name} value={name}>
                {name === me ? t(lang, "rep.me", { name }) : name}
                {countOf(facets?.assignees, name) !== undefined ? ` (${countOf(facets?.assignees, name)})` : ""}
              </option>
            ))}
            {filter.assignee !== undefined && filter.assignee !== "none" && !people.includes(filter.assignee) && (
              <option value={filter.assignee}>{filter.assignee}</option>
            )}
          </select>,
        )}
        {field(
          "f-label",
          t(lang, "rep.label"),
          <select
            id="f-label"
            value={filter.label ?? ""}
            onChange={(event) => {
              set({ label: event.target.value || undefined });
            }}
          >
            <option value="">{t(lang, "rep.any")}</option>
            {labels.map((c) => (
              <option key={c.value} value={c.value}>
                {c.value} ({c.count})
              </option>
            ))}
            {filter.label !== undefined && !labels.some((c) => c.value === filter.label) && (
              <option value={filter.label}>{filter.label}</option>
            )}
          </select>,
        )}
        {field(
          "f-gateway",
          t(lang, "rep.gateway"),
          <select
            id="f-gateway"
            value={filter.gateway ?? ""}
            onChange={(event) => {
              set({ gateway: event.target.value || undefined });
            }}
          >
            <option value="">{t(lang, "rep.any")}</option>
            {(facets?.gateways ?? []).map((g) =>
              g.value === null ? null : (
                <option key={g.value} value={g.value}>
                  {g.value} ({g.count})
                </option>
              ),
            )}
            {filter.gateway && !facets?.gateways.some((g) => g.value === filter.gateway) && (
              <option value={filter.gateway}>{filter.gateway}</option>
            )}
          </select>,
        )}
        {field(
          "f-channel",
          t(lang, "rep.channel"),
          <select
            id="f-channel"
            value={filter.channel ?? ""}
            onChange={(event) => {
              const v = event.target.value;
              set({ channel: v === "gateway" || v === "direct" ? v : undefined });
            }}
          >
            <option value="">{t(lang, "rep.any")}</option>
            <option value="gateway">{t(lang, "rep.channelGateway")}</option>
            <option value="direct">{t(lang, "rep.channelDirect")}</option>
          </select>,
        )}
        {field(
          "f-source",
          t(lang, "rep.source"),
          <select
            id="f-source"
            title={t(lang, "rep.sourceTitle")}
            value={filter.source ?? ""}
            onChange={(event) => {
              set({ source: event.target.value || undefined });
            }}
          >
            <option value="">{t(lang, "rep.any")}</option>
            <option value="none">
              {t(lang, "rep.noSource")}
              {countOf(facets?.sources, null) === undefined ? "" : ` (${countOf(facets?.sources, null)})`}
            </option>
            {sources.map((c) => (
              <option key={c.value} value={c.value}>
                {c.value} ({c.count})
              </option>
            ))}
            {sources.map((c) => (
              <option key={`!${c.value}`} value={`!${c.value}`}>
                {sourceLabel(lang, `!${c.value}`)}
              </option>
            ))}
            {filter.source !== undefined &&
              filter.source !== "none" &&
              !sources.some((c) => c.value === filter.source || `!${c.value}` === filter.source) && (
                <option value={filter.source}>{sourceLabel(lang, filter.source)}</option>
              )}
          </select>,
        )}
        {field(
          "f-reporter",
          t(lang, "rep.reporter"),
          <>
            <input
              id="f-reporter"
              list="f-reporters"
              spellCheck={false}
              placeholder={t(lang, "rep.reporterHint")}
              defaultValue={filter.reporter ?? ""}
              key={filter.reporter ?? ""}
              onBlur={(event) => {
                const value = event.target.value.trim();
                if (value !== (filter.reporter ?? "")) set({ reporter: value || undefined });
              }}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  const value = event.currentTarget.value.trim();
                  set({ reporter: value || undefined });
                }
              }}
            />
            <datalist id="f-reporters">
              {(facets?.reporters ?? []).map((r) =>
                r.value === null ? null : (
                  <option key={r.value} value={r.value}>
                    {t(lang, "rep.reportsN", { n: r.count })}
                  </option>
                ),
              )}
            </datalist>
          </>,
        )}
        {field(
          "f-from",
          t(lang, "rep.from"),
          <input
            id="f-from"
            type="date"
            value={filter.from ?? ""}
            onChange={(event) => {
              set({ from: event.target.value || undefined });
            }}
          />,
        )}
        {field(
          "f-to",
          t(lang, "rep.to"),
          <input
            id="f-to"
            type="date"
            value={filter.to ?? ""}
            onChange={(event) => {
              set({ to: event.target.value || undefined });
            }}
          />,
        )}
        {field(
          "f-record",
          t(lang, "rep.record"),
          <select
            id="f-record"
            value={filter.has_record === undefined ? "" : String(filter.has_record)}
            onChange={(event) => {
              const v = event.target.value;
              set({ has_record: v === "" ? undefined : v === "true" });
            }}
          >
            <option value="">{t(lang, "rep.any")}</option>
            <option value="true">{t(lang, "rep.withRecord")}</option>
            <option value="false">{t(lang, "rep.withoutRecord")}</option>
          </select>,
        )}
        {field(
          "f-sort",
          t(lang, "rep.sort"),
          <span className="sort-pick">
            <select
              id="f-sort"
              value={filter.sort ?? "received"}
              onChange={(event) => {
                const sort = SORTS.find((s) => s === event.target.value) ?? "received";
                set({ sort, dir: firstDir(sort) });
              }}
            >
              {SORTS.map((s) => (
                <option key={s} value={s}>
                  {sortLabel(lang, s)}
                </option>
              ))}
            </select>
            <button
              type="button"
              className="quiet"
              aria-label={filter.dir === "asc" ? t(lang, "rep.ascending") : t(lang, "rep.descending")}
              title={filter.dir === "asc" ? t(lang, "rep.ascending") : t(lang, "rep.descending")}
              onClick={() => {
                set({ dir: filter.dir === "asc" ? "desc" : "asc" });
              }}
            >
              {filter.dir === "asc" ? "↑" : "↓"}
            </button>
          </span>,
        )}
        <div className="field check-field">
          <label>
            <input
              type="checkbox"
              checked={filter.unseen === true}
              onChange={(event) => {
                set({ unseen: event.target.checked ? true : undefined });
              }}
            />{" "}
            {t(lang, "rep.onlyUnread")}
          </label>
        </div>
      </div>
    </form>
  );
}

/** One active filter, as a pill that removes it. */
interface Pill {
  key: string;
  text: string;
  change: FilterChange;
}

/** The active filters beyond the view, as pills. `viewOwns` says which the view itself sets. */
export function pillsOf(lang: Lang, filter: Filter, viewOwns: ReadonlySet<keyof Filter>): Pill[] {
  const pills: Pill[] = [];
  const add = (key: keyof Filter, text: string, change: FilterChange) => {
    if (!viewOwns.has(key)) pills.push({ key: `${key}:${text}`, text, change });
  };
  for (const s of filter.status ?? []) {
    add("status", `${t(lang, "rep.status")}: ${statusLabel(lang, s)}`, { status: toggled(filter.status, s, STATUSES) });
  }
  for (const k of filter.kind ?? []) {
    add("kind", `${t(lang, "rep.kind")}: ${kindLabel(lang, k)}`, { kind: toggled(filter.kind, k, KINDS) });
  }
  const priorities = [...PRIORITIES, "none"] as const;
  for (const p of filter.priority ?? []) {
    add("priority", `${t(lang, "rep.priority")}: ${p === "none" ? t(lang, "rep.noPriority") : priorityLabel(lang, p)}`, {
      priority: toggled(filter.priority, p, priorities),
    });
  }
  if (filter.assignee !== undefined) {
    add(
      "assignee",
      `${t(lang, "rep.assignee")}: ${filter.assignee === "none" ? t(lang, "rep.nobody") : filter.assignee}`,
      { assignee: undefined },
    );
  }
  if (filter.label !== undefined) add("label", `${t(lang, "rep.label")}: ${filter.label}`, { label: undefined });
  if (filter.gateway !== undefined) add("gateway", `${t(lang, "rep.gateway")}: ${filter.gateway}`, { gateway: undefined });
  if (filter.channel !== undefined) {
    add(
      "channel",
      filter.channel === "direct" ? t(lang, "rep.channelDirect") : t(lang, "rep.channelGateway"),
      { channel: undefined },
    );
  }
  if (filter.source !== undefined) {
    add("source", `${t(lang, "rep.source")}: ${sourceLabel(lang, filter.source)}`, { source: undefined });
  }
  if (filter.reporter !== undefined) add("reporter", `${t(lang, "rep.reporter")}: ${filter.reporter}`, { reporter: undefined });
  if (filter.game_id !== undefined) add("game_id", `${t(lang, "rep.game")}: ${filter.game_id}`, { game_id: undefined });
  if (filter.signature !== undefined) {
    add("signature", `${t(lang, "rep.signature")}: ${filter.signature.slice(0, 8)}`, { signature: undefined });
  }
  if (filter.duplicate_of !== undefined) {
    add("duplicate_of", t(lang, "rep.duplicatesOf", { id: shortId(filter.duplicate_of) }), { duplicate_of: undefined });
  }
  if (filter.q !== undefined) add("q", `“${filter.q}”`, { q: undefined });
  if (filter.from !== undefined) add("from", `${t(lang, "rep.from")}: ${filter.from}`, { from: undefined });
  if (filter.to !== undefined) add("to", `${t(lang, "rep.to")}: ${filter.to}`, { to: undefined });
  if (filter.has_record !== undefined) {
    add("has_record", filter.has_record ? t(lang, "rep.withRecord") : `${t(lang, "rep.record")}: ${t(lang, "rep.withoutRecord")}`, {
      has_record: undefined,
    });
  }
  if (filter.unseen === true) add("unseen", t(lang, "rep.onlyUnread"), { unseen: undefined });
  if (filter.trash === true) add("trash", t(lang, "rep.inTrashFilter"), { trash: undefined });
  return pills;
}

export function Pills({
  lang,
  pills,
  set,
  onClear,
}: {
  lang: Lang;
  pills: readonly Pill[];
  set: (change: FilterChange) => void;
  onClear: () => void;
}) {
  if (pills.length === 0) return null;
  return (
    <ul className="pills" aria-label={t(lang, "rep.filters")}>
      {pills.map((pill) => (
        <li key={pill.key}>
          <span>{pill.text}</span>
          <button
            type="button"
            className="quiet"
            aria-label={t(lang, "rep.removeFilter", { what: pill.text })}
            onClick={() => {
              set(pill.change);
            }}
          >
            ×
          </button>
        </li>
      ))}
      {pills.length > 0 && (
        <li className="pills-clear">
          <button type="button" className="link small" onClick={onClear}>
            {t(lang, "rep.clear")}
          </button>
        </li>
      )}
    </ul>
  );
}
