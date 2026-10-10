// The list of reports (`/`): views and filters in the query string so a
// view is a link, a cursor the keyboard moves, a drawer that opens a
// report beside the list, a selection for changing several at once, a
// quiet refresh that announces new arrivals instead of shifting the rows
// under the cursor, and an export of what is shown.

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
  type MouseEvent,
  type ReactNode,
} from "react";

import { useVisibleInterval } from "./admin/shared";
import {
  api,
  filterQuery,
  KINDS,
  PAGE_SIZE,
  parseFilter,
  Refused,
  SignedOut,
  STATUSES,
  type Facets,
  type Filter,
  type Listing,
  type Status,
  type Summary,
} from "./api";
import { firstLine } from "./github";
import { formatBytes } from "./format";
import { formatCount, kindLabel, statusLabel, t, type Key, type Lang } from "./i18n";
import { Drawer } from "./reports/Drawer";
import { download, toCsv } from "./reports/csv";
import { Reporter, Time, toast } from "./reports/pieces";
import { isSeen, markAllSeen, markSeen } from "./reports/seen";
import { navigate } from "./router";

/** A change to the filters; `undefined` clears one. */
type FilterChange = { [K in keyof Filter]?: Filter[K] | undefined };

/** How often the list is asked again, quietly, while the page is visible. */
export const LIST_REFRESH_MS = 30_000;

/** The views at the top: each is a status, or none for all. */
const VIEWS: { id: string; status: Status | undefined; label: Key }[] = [
  { id: "inbox", status: "new", label: "rep.view.inbox" },
  { id: "triaged", status: "triaged", label: "rep.view.triaged" },
  { id: "in_progress", status: "in_progress", label: "rep.view.in_progress" },
  { id: "resolved", status: "resolved", label: "rep.view.resolved" },
  { id: "all", status: undefined, label: "rep.view.all" },
];

/** Whether a key press belongs to a field rather than to the list. */
function typing(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName))
  );
}

/** `filter` with `change` applied, back on the first page. */
export function applyChange(filter: Filter, change: FilterChange): Filter {
  const params = new URLSearchParams(filterQuery(filter));
  params.delete("offset");
  for (const [key, value] of Object.entries(change)) {
    if (value === undefined || value === "") params.delete(key);
    else params.set(key, String(value));
  }
  return parseFilter(params.toString());
}

function go(filter: Filter) {
  const query = filterQuery(filter);
  navigate(query ? `/?${query}` : "/", true);
}

/** The filters beyond the view and the text, for the "Filters (n)" count. */
function extraFilters(filter: Filter): number {
  return [filter.kind, filter.gateway, filter.reporter, filter.from, filter.to, filter.has_record].filter(
    (v) => v !== undefined,
  ).length;
}

/** Whether a wide screen shows the drawer beside the list, else over it. */
function wide(): boolean {
  return typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia("(min-width: 1100px)").matches
    : true;
}

function openPage(id: string): void {
  markSeen(id);
  navigate(`/r/${id}`);
}

const linkOf = (id: string) => `${window.location.origin}/r/${id}`;

const field = (id: string, label: string, control: ReactNode) => (
  <div className="field">
    <label htmlFor={id}>{label}</label>
    {control}
  </div>
);

function Shortcuts({ lang, onClose }: { lang: Lang; onClose: () => void }) {
  const rows: [string, Key][] = [
    ["j / k, ↓ / ↑", "rep.key.move"],
    ["Enter", "rep.key.open"],
    ["o", "rep.key.page"],
    ["x", "rep.key.select"],
    ["a", "rep.key.selectAll"],
    ["1 – 6", "rep.key.status"],
    ["/", "rep.key.search"],
    ["r", "rep.key.refresh"],
    ["c", "rep.key.copy"],
    ["e", "rep.key.export"],
    ["Esc", "rep.key.close"],
    ["?", "rep.key.help"],
  ];
  return (
    <dialog className="shortcuts" open aria-label={t(lang, "rep.shortcuts")}>
      <div className="panel-head">
        <h2>{t(lang, "rep.shortcuts")}</h2>
        <button type="button" className="quiet" onClick={onClose} aria-label={t(lang, "rep.close")}>
          ×
        </button>
      </div>
      <dl className="kv">
        {rows.map(([keys, label]) => (
          <div className="kv-row" key={keys}>
            <dt>
              <kbd>{keys}</kbd>
            </dt>
            <dd>{t(lang, label)}</dd>
          </div>
        ))}
      </dl>
    </dialog>
  );
}

export function ReportList({ search, lang = "en" }: { search: string; lang?: Lang }) {
  const filter = useMemo(() => parseFilter(search), [search]);
  const [listing, setListing] = useState<Listing | null>(null);
  const [arrived, setArrived] = useState<Listing | null>(null);
  const [facets, setFacets] = useState<Facets | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [cursor, setCursor] = useState(0);
  const [checked, setChecked] = useState<ReadonlySet<string>>(new Set());
  const [drawer, setDrawer] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [updatedAt, setUpdatedAt] = useState<number | null>(null);
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [help, setHelp] = useState(false);
  const [seenTick, setSeenTick] = useState(0);
  const [bulkStatus, setBulkStatus] = useState("");
  const [busy, setBusy] = useState(false);
  const searchBox = useRef<HTMLInputElement>(null);
  const rows = useRef<(HTMLTableRowElement | null)[]>([]);
  const current = useRef<Listing | null>(null);
  useEffect(() => {
    current.current = listing;
  }, [listing]);

  // A new filter is a new list: from the top, nothing selected, drawer
  // shut. Derived from the search during the render, as React asks.
  const [prevSearch, setPrevSearch] = useState(search);
  if (search !== prevSearch) {
    setPrevSearch(search);
    setListing(null);
    setArrived(null);
    setCursor(0);
    setChecked(new Set());
    setDrawer(null);
  }

  const describe = useCallback(
    (e: unknown) => (e instanceof Refused ? e.message : t(lang, "admin.noAnswer")),
    [lang],
  );

  /**
   * Asks for the list. `quiet` keeps the rows where they are when new
   * reports arrived (they are announced instead), so the cursor and the
   * drawer stay on what the admin was reading.
   */
  const load = useCallback(
    (quiet: boolean) => {
      api
        .reports(filter)
        .then((fresh) => {
          setError(null);
          setUpdatedAt(Date.now());
          const before = current.current;
          const known = new Set(before?.reports.map((r) => r.id) ?? []);
          const unseen = fresh.reports.some((r) => !known.has(r.id));
          if (quiet && before !== null && unseen) {
            // Changes to rows already shown are applied; new rows wait.
            setListing({
              total: before.total,
              reports: before.reports.map((r) => fresh.reports.find((f) => f.id === r.id) ?? r),
            });
            setArrived(fresh);
          } else {
            setListing(fresh);
            setArrived(null);
            if (!quiet) setCursor((c) => Math.min(c, Math.max(fresh.reports.length - 1, 0)));
          }
        })
        .catch((e: unknown) => {
          if (e instanceof SignedOut) return;
          const why = describe(e);
          if (quiet) toast("error", why);
          else setError(why);
        })
        .finally(() => {
          setRefreshing(false);
        });
    },
    [filter, describe],
  );

  // A new filter asks at once; the interval asks again quietly while there
  // is a list to refresh (never at mount, where the effect already asks).
  useEffect(() => {
    load(false);
  }, [load]);
  useVisibleInterval(
    useCallback(() => {
      if (current.current !== null) load(true);
    }, [load]),
    LIST_REFRESH_MS,
  );

  useEffect(() => {
    api
      .facets()
      .then(setFacets)
      .catch(() => {
        // The filters work without their suggestions.
      });
  }, []);

  /** The Refresh button and `r`: asked at once, with the spinner on. */
  const refresh = () => {
    setRefreshing(true);
    load(false);
  };

  const reports = useMemo(() => listing?.reports ?? [], [listing]);
  const under = reports[cursor];

  const showArrived = () => {
    if (arrived === null) return;
    setListing(arrived);
    setArrived(null);
    setCursor(0);
  };

  const openDrawer = useCallback((id: string) => {
    markSeen(id);
    setSeenTick((n) => n + 1);
    setDrawer(id);
  }, []);

  /** Moves the cursor by `delta`; a drawer open beside the list follows it. */
  const move = (delta: number) => {
    if (reports.length === 0) return;
    const next = Math.max(0, Math.min(cursor + delta, reports.length - 1));
    setCursor(next);
    const report = reports[next];
    if (drawer !== null && report && drawer !== report.id && wide()) openDrawer(report.id);
  };

  const toggle = (id: string) => {
    setChecked((was) => {
      const next = new Set(was);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const copyText = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast("ok", t(lang, "rep.copied"));
    } catch {
      toast("error", t(lang, "rep.copyFailed"));
    }
  };

  /** The reports a bulk action works on: the checked ones, else the one under the cursor. */
  const targets = (): Summary[] =>
    checked.size > 0 ? reports.filter((r) => checked.has(r.id)) : under ? [under] : [];

  const exportCsv = () => {
    const chosen = targets().length > 1 ? targets() : reports;
    if (chosen.length === 0) return;
    const day = new Date().toISOString().slice(0, 10);
    download(`baylee-reports-${day}.csv`, toCsv(chosen));
    toast("ok", t(lang, "rep.exported", { n: chosen.length }));
  };

  /** Sets `status` on each of `chosen`, one request each, every one audited. */
  const setStatusOn = async (chosen: Summary[], status: Status) => {
    if (chosen.length === 0 || busy) return;
    setBusy(true);
    let failed = 0;
    let why = "";
    const changed = new Map<string, Summary>();
    for (const report of chosen) {
      try {
        const updated = await api.change(report.id, { status });
        changed.set(updated.id, updated);
      } catch (e: unknown) {
        if (e instanceof SignedOut) {
          setBusy(false);
          return;
        }
        failed += 1;
        why = describe(e);
      }
    }
    setListing((was) =>
      was === null ? was : { ...was, reports: was.reports.map((r) => changed.get(r.id) ?? r) },
    );
    setBusy(false);
    setBulkStatus("");
    if (failed > 0) {
      toast("error", t(lang, "rep.bulkFailed", { failed, total: chosen.length, why }));
    } else {
      toast("ok", t(lang, "rep.bulkDone", { n: chosen.length, status: statusLabel(lang, status) }));
    }
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (event.key === "Escape") {
        if (help) setHelp(false);
        else if (drawer !== null) setDrawer(null);
        else if (checked.size > 0) setChecked(new Set());
        else if (typing(event.target)) (event.target as HTMLElement).blur();
        return;
      }
      if (typing(event.target)) return;
      const key = event.key;
      const stop = () => {
        event.preventDefault();
      };
      if (key === "/") {
        stop();
        searchBox.current?.focus();
      } else if ((key === "j" || key === "ArrowDown") && reports.length > 0) {
        stop();
        move(1);
      } else if ((key === "k" || key === "ArrowUp") && reports.length > 0) {
        stop();
        move(-1);
      } else if (key === "Enter" && under) {
        stop();
        openDrawer(under.id);
      } else if (key === "o" && under) {
        stop();
        openPage(under.id);
      } else if (key === "x" && under) {
        stop();
        toggle(under.id);
      } else if (key === "a" && reports.length > 0) {
        stop();
        setChecked((was) => (was.size === reports.length ? new Set() : new Set(reports.map((r) => r.id))));
      } else if (key === "r") {
        stop();
        refresh();
      } else if (key === "c" && under) {
        stop();
        void copyText(linkOf(under.id));
      } else if (key === "e") {
        stop();
        exportCsv();
      } else if (key === "?") {
        stop();
        setHelp((h) => !h);
      } else if (/^[1-6]$/.test(key)) {
        const status = STATUSES[Number(key) - 1];
        if (status) {
          stop();
          void setStatusOn(targets(), status);
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  });

  useEffect(() => {
    rows.current[cursor]?.scrollIntoView?.({ block: "nearest" });
  }, [cursor]);

  // Any change of filter starts again at the first page.
  const set = (change: FilterChange) => {
    go(applyChange(filter, change));
  };

  // The search box is applied on Enter or when it loses focus, not per key.
  const applySearch = () => {
    const q = searchBox.current?.value.trim() ?? "";
    if (q !== (filter.q ?? "")) set({ q: q === "" ? undefined : q });
  };

  const submitSearch = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    applySearch();
  };

  const offset = filter.offset ?? 0;
  const total = listing?.total ?? 0;
  const anyFilter = filterQuery({ ...filter, offset: 0 }) !== "";
  const extras = extraFilters(filter);
  const view = VIEWS.find((v) => v.status === filter.status)?.id ?? "custom";
  const allChecked = reports.length > 0 && reports.every((r) => checked.has(r.id));
  const unreadHere = reports.filter((r) => r.status === "new" && !isSeen(r.id)).length;
  const n = (v: number) => formatCount(lang, v);
  // `seenTick` only re-renders after a report was opened; its value is not read.
  void seenTick;

  const onRowClick = (event: MouseEvent<HTMLTableRowElement>, report: Summary, index: number) => {
    const target = event.target;
    if (target instanceof HTMLElement && target.closest("a, button, input, select, label")) return;
    setCursor(index);
    openDrawer(report.id);
  };

  return (
    <main className={`list-page${drawer !== null ? " with-drawer" : ""}`}>
      <div className="list-main">
        <div className="list-toolbar">
          <fieldset className="views segmented">
            <legend className="sr-only">{t(lang, "rep.views")}</legend>
            {VIEWS.map((v) => (
              <button
                key={v.id}
                type="button"
                aria-pressed={view === v.id}
                onClick={() => {
                  set({ status: v.status });
                }}
              >
                {t(lang, v.label)}
                {v.status !== undefined && facets !== null && (
                  <span className="seg-count">
                    {n(facets.statuses.find((s) => s.value === v.status)?.count ?? 0)}
                  </span>
                )}
              </button>
            ))}
          </fieldset>
          <form className="search-form" onSubmit={submitSearch}>
            <label htmlFor="f-q" className="sr-only">
              {t(lang, "rep.searchLabel")}
            </label>
            <input
              id="f-q"
              ref={searchBox}
              type="search"
              placeholder={t(lang, "rep.search")}
              key={filter.q ?? ""}
              defaultValue={filter.q ?? ""}
              onBlur={applySearch}
            />
          </form>
          <button
            type="button"
            className={extras > 0 ? "accent-outline" : undefined}
            aria-expanded={filtersOpen || extras > 0}
            aria-controls="list-filters"
            onClick={() => {
              setFiltersOpen((o) => !o);
            }}
          >
            {extras > 0 ? t(lang, "rep.filtersOn", { n: extras }) : t(lang, "rep.filters")}
          </button>
          <span className="spacer" />
          <span className="freshness muted small" aria-live="polite">
            {updatedAt !== null &&
              t(lang, "rep.updated", {
                time: new Intl.DateTimeFormat(lang === "de" ? "de-DE" : "en-GB", { timeStyle: "short" }).format(
                  new Date(updatedAt),
                ),
              })}
          </span>
          <button
            type="button"
            className={refreshing ? "refreshing" : undefined}
            aria-busy={refreshing}
            disabled={refreshing}
            onClick={refresh}
          >
            <span className={`spin${refreshing ? " on" : ""}`} aria-hidden="true" />
            {refreshing ? t(lang, "rep.refreshing") : t(lang, "rep.refresh")}
          </button>
          <button
            type="button"
            className="quiet help-key"
            aria-label={t(lang, "rep.shortcuts")}
            title={t(lang, "rep.shortcuts")}
            onClick={() => {
              setHelp((h) => !h);
            }}
          >
            ?
          </button>
        </div>

        <form
          id="list-filters"
          className="filters"
          hidden={!filtersOpen && extras === 0}
          onSubmit={submitSearch}
          aria-label={t(lang, "rep.filters")}
        >
          {field(
            "f-kind",
            t(lang, "rep.kind"),
            <select
              id="f-kind"
              value={filter.kind ?? ""}
              onChange={(event) => {
                set({ kind: KINDS.find((k) => k === event.target.value) });
              }}
            >
              <option value="">{t(lang, "rep.any")}</option>
              {KINDS.map((kind) => (
                <option key={kind} value={kind}>
                  {kindLabel(lang, kind)}
                </option>
              ))}
            </select>,
          )}
          {field(
            "f-status",
            t(lang, "rep.status"),
            <select
              id="f-status"
              value={filter.status ?? ""}
              onChange={(event) => {
                set({ status: STATUSES.find((s) => s === event.target.value) });
              }}
            >
              <option value="">{t(lang, "rep.any")}</option>
              {STATUSES.map((status) => (
                <option key={status} value={status}>
                  {statusLabel(lang, status)}
                </option>
              ))}
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
              {(facets?.gateways ?? []).map((g) => (
                <option key={g.value} value={g.value}>
                  {g.value} ({g.count})
                </option>
              ))}
              {filter.gateway && !facets?.gateways.some((g) => g.value === filter.gateway) && (
                <option value={filter.gateway}>{filter.gateway}</option>
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
                {(facets?.reporters ?? []).map((r) => (
                  <option key={r.value} value={r.value}>
                    {t(lang, "rep.reportsN", { n: r.count })}
                  </option>
                ))}
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
          <div className="field actions">
            <button type="submit" className="primary">
              {t(lang, "rep.apply")}
            </button>
            {anyFilter && (
              <button
                type="button"
                className="quiet"
                onClick={() => {
                  go({});
                }}
              >
                {t(lang, "rep.clear")}
              </button>
            )}
          </div>
        </form>

        {facets && facets.reporters.length > 1 && (
          <details className="reporters">
            <summary>{t(lang, "rep.reportersTop")}</summary>
            <ul>
              {facets.reporters.map((r) => (
                <li key={r.value}>
                  <Reporter
                    lang={lang}
                    pseudonym={r.value}
                    onClick={() => {
                      set({ reporter: r.value });
                    }}
                  />{" "}
                  <span className="muted">{r.count}</span>
                </li>
              ))}
            </ul>
          </details>
        )}

        {error !== null && (
          <p className="error" role="alert">
            {error}
          </p>
        )}

        {arrived !== null && (
          <output className="arrived">
            <span>
              {t(lang, "rep.arrived", {
                n: arrived.reports.filter((r) => !reports.some((k) => k.id === r.id)).length,
              })}
            </span>
            <button type="button" className="primary" onClick={showArrived}>
              {t(lang, "rep.show")}
            </button>
          </output>
        )}

        {checked.size > 0 ? (
          <div className="bulk" role="toolbar" aria-label={t(lang, "rep.selected", { n: checked.size })}>
            <strong>{t(lang, "rep.selected", { n: n(checked.size) })}</strong>
            <label htmlFor="bulk-status" className="sr-only">
              {t(lang, "rep.setStatus")}
            </label>
            <select
              id="bulk-status"
              value={bulkStatus}
              disabled={busy}
              onChange={(event) => {
                const status = STATUSES.find((s) => s === event.target.value);
                setBulkStatus(event.target.value);
                if (status) void setStatusOn(targets(), status);
              }}
            >
              <option value="">{t(lang, "rep.setStatus")}…</option>
              {STATUSES.map((s) => (
                <option key={s} value={s}>
                  {statusLabel(lang, s)}
                </option>
              ))}
            </select>
            <button type="button" onClick={() => void copyText([...checked].join("\n"))}>
              {t(lang, "rep.copyIds")}
            </button>
            <button type="button" onClick={exportCsv}>
              {t(lang, "rep.exportCsv")}
            </button>
            <button
              type="button"
              onClick={() => {
                markAllSeen([...checked]);
                setSeenTick((k) => k + 1);
              }}
            >
              {t(lang, "rep.markRead")}
            </button>
            <button
              type="button"
              className="quiet"
              onClick={() => {
                setChecked(new Set());
              }}
            >
              {t(lang, "rep.deselect")}
            </button>
          </div>
        ) : (
          <p className="count muted" aria-live="polite">
            {listing === null
              ? t(lang, "rep.loading")
              : total === 0
                ? t(lang, "rep.none")
                : t(lang, "rep.range", { from: n(offset + 1), to: n(offset + reports.length), total: n(total) })}
            {unreadHere > 0 && <span className="unread-count"> · {unreadHere} {t(lang, "rep.unread")}</span>}
            {reports.length > 0 && (
              <>
                {" "}
                <button type="button" className="link small" onClick={exportCsv}>
                  {t(lang, "rep.exportCsv")}
                </button>
              </>
            )}
          </p>
        )}

        {reports.length > 0 && (
          <div className="table-wrap">
            <table className="reports">
              <thead>
                <tr>
                  <th scope="col" className="check">
                    <input
                      type="checkbox"
                      aria-label={t(lang, "rep.selectAll")}
                      checked={allChecked}
                      onChange={() => {
                        setChecked(allChecked ? new Set() : new Set(reports.map((r) => r.id)));
                      }}
                    />
                  </th>
                  <th scope="col">{t(lang, "rep.col.received")}</th>
                  <th scope="col">{t(lang, "rep.col.kind")}</th>
                  <th scope="col">{t(lang, "rep.col.status")}</th>
                  <th scope="col" className="wide">
                    {t(lang, "rep.col.text")}
                  </th>
                  <th scope="col">{t(lang, "rep.col.gateway")}</th>
                  <th scope="col">{t(lang, "rep.col.reporter")}</th>
                  <th scope="col">{t(lang, "rep.col.record")}</th>
                  <th scope="col">{t(lang, "rep.col.issue")}</th>
                </tr>
              </thead>
              <tbody>
                {reports.map((report, index) => {
                  const unread = report.status === "new" && !isSeen(report.id);
                  const classes = [
                    index === cursor ? "selected" : "",
                    unread ? "unread" : "",
                    checked.has(report.id) ? "checked" : "",
                    drawer === report.id ? "open" : "",
                  ]
                    .filter(Boolean)
                    .join(" ");
                  return (
                    <tr
                      key={report.id}
                      ref={(el) => {
                        rows.current[index] = el;
                      }}
                      className={classes || undefined}
                      aria-selected={index === cursor}
                      onClick={(event) => {
                        onRowClick(event, report, index);
                      }}
                    >
                      <td className="check">
                        <input
                          type="checkbox"
                          aria-label={t(lang, "rep.select", { what: firstLine(report.text, 40) || report.id })}
                          checked={checked.has(report.id)}
                          onChange={() => {
                            toggle(report.id);
                          }}
                        />
                      </td>
                      <td className="nowrap">
                        {unread && <span className="unread-dot" title={t(lang, "rep.unread")} />}
                        <Time lang={lang} iso={report.created_at} />
                      </td>
                      <td>
                        <span className={`tag kind-${report.kind}`}>{kindLabel(lang, report.kind)}</span>
                      </td>
                      <td>
                        <span className={`tag status-${report.status}`}>{statusLabel(lang, report.status)}</span>
                      </td>
                      <td className="wide">
                        <a
                          href={`/r/${report.id}`}
                          onClick={(event) => {
                            if (event.metaKey || event.ctrlKey || event.shiftKey) return;
                            event.preventDefault();
                            openPage(report.id);
                          }}
                        >
                          {firstLine(report.text, 120) || <span className="muted">{t(lang, "rep.noText")}</span>}
                        </a>
                      </td>
                      <td>{report.channel === "direct" ? "direct" : report.gateway}</td>
                      <td>
                        <Reporter
                          lang={lang}
                          pseudonym={report.reporter}
                          onClick={() => {
                            set({ reporter: report.reporter });
                          }}
                        />
                      </td>
                      <td className="nowrap">
                        {report.has_record ? (
                          <>
                            {formatBytes(report.record_bytes)}
                            {report.record_origin === "client" && <span className="muted"> (client)</span>}
                          </>
                        ) : (
                          <span className="muted">—</span>
                        )}
                      </td>
                      <td>
                        {report.issue_number !== null && report.issue_url !== null ? (
                          <a href={report.issue_url} target="_blank" rel="noopener noreferrer">
                            #{report.issue_number}
                          </a>
                        ) : (
                          <span className="muted">—</span>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}

        {total > PAGE_SIZE && (
          <nav className="pager" aria-label="Pages">
            <button
              type="button"
              disabled={offset === 0}
              onClick={() => {
                go({ ...filter, offset: Math.max(offset - PAGE_SIZE, 0) });
              }}
            >
              ← Newer
            </button>
            <button
              type="button"
              disabled={offset + PAGE_SIZE >= total}
              onClick={() => {
                go({ ...filter, offset: offset + PAGE_SIZE });
              }}
            >
              Older →
            </button>
          </nav>
        )}
      </div>

      {drawer !== null && (
        <Drawer
          key={drawer}
          id={drawer}
          lang={lang}
          onClose={() => {
            setDrawer(null);
          }}
          onChanged={(updated) => {
            setListing((was) =>
              was === null ? was : { ...was, reports: was.reports.map((r) => (r.id === updated.id ? updated : r)) },
            );
          }}
          onDeleted={(id) => {
            setDrawer(null);
            setListing((was) =>
              was === null ? was : { total: was.total - 1, reports: was.reports.filter((r) => r.id !== id) },
            );
          }}
        />
      )}

      {help && (
        <Shortcuts
          lang={lang}
          onClose={() => {
            setHelp(false);
          }}
        />
      )}
    </main>
  );
}
