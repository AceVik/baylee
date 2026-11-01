// The list of reports (`/`): views, filters, sort and page in the query
// string so a list is a link, saved views kept on the service, a cursor
// the keyboard moves, a drawer that opens a report beside the list, a
// selection changed in one request, the trash with an undo, crash groups,
// a quiet refresh that announces new arrivals instead of shifting the rows
// under the cursor, and an export of what is shown or of all that matches.

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
  type MouseEvent,
} from "react";

import { useVisibleInterval } from "./admin/shared";
import {
  api,
  DONE_STATUSES,
  filterQuery,
  isSourceName,
  OPEN_STATUSES,
  PAGE_SIZE,
  PAGE_SIZES,
  parseFilter,
  Refused,
  SignedOut,
  STATUSES,
  TRASH_DAYS,
  type BulkAction,
  type BulkChange,
  type Facets,
  type Filter,
  type Listing,
  type Sort,
  type Summary,
} from "./api";
import { firstLine } from "./github";
import { formatBytes } from "./format";
import { formatCount, kindLabel, localeOf, sortLabel, statusLabel, t, type Key, type Lang } from "./i18n";
import { BulkBar } from "./reports/BulkBar";
import { CrashGroups } from "./reports/CrashGroups";
import { Drawer } from "./reports/Drawer";
import { download, toCsv } from "./reports/csv";
import { FilterPanel, firstDir, Pills, pillsOf, type FilterChange } from "./reports/ListFilters";
import { Labels, PriorityTag, Reporter, shortId, Time, toast } from "./reports/pieces";
import { SavedViews } from "./reports/SavedViews";
import { navigate } from "./router";

/** How often the list is asked again, quietly, while the page is visible. */
export const LIST_REFRESH_MS = 30_000;

type ViewId = "inbox" | "open" | "mine" | "unassigned" | "done" | "all" | "trash";

/** The views at the top: each sets the status, the assignee and the trash, and leaves the rest. */
const VIEWS: { id: ViewId; label: Key; filter: (me: string) => Filter }[] = [
  { id: "inbox", label: "rep.view.inbox", filter: () => ({ status: ["new"] }) },
  { id: "open", label: "rep.view.open", filter: () => ({ status: [...OPEN_STATUSES] }) },
  { id: "mine", label: "rep.view.mine", filter: (me) => ({ status: [...OPEN_STATUSES], assignee: me }) },
  { id: "unassigned", label: "rep.view.unassigned", filter: () => ({ status: [...OPEN_STATUSES], assignee: "none" }) },
  { id: "done", label: "rep.view.done", filter: () => ({ status: [...DONE_STATUSES] }) },
  { id: "all", label: "rep.view.all", filter: () => ({}) },
  { id: "trash", label: "rep.view.trash", filter: () => ({ trash: true }) },
];

function sameSet(a: readonly string[] | undefined, b: readonly string[] | undefined): boolean {
  const x = new Set(a ?? []);
  const y = new Set(b ?? []);
  return x.size === y.size && [...x].every((v) => y.has(v));
}

/** The view `filter` is in, if any. */
function viewOf(filter: Filter, me: string): ViewId | "custom" {
  for (const view of VIEWS) {
    const f = view.filter(me);
    if (
      sameSet(filter.status, f.status) &&
      filter.assignee === f.assignee &&
      (filter.trash === true) === (f.trash === true)
    ) {
      return view.id;
    }
  }
  return "custom";
}

/** Whether a key press belongs to a field rather than to the list. */
function typing(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName))
  );
}

/** `filter` with `change` applied, back on the first page. */
export function applyChange(filter: Filter, change: FilterChange): Filter {
  const next: Filter = { ...filter, offset: undefined };
  for (const [key, value] of Object.entries(change) as [keyof Filter, unknown][]) {
    if (value === undefined || value === "" || (Array.isArray(value) && value.length === 0)) {
      delete next[key];
    } else {
      (next as Record<string, unknown>)[key] = value;
    }
  }
  return parseFilter(filterQuery(next));
}

function go(filter: Filter) {
  const query = filterQuery(filter);
  navigate(query ? `/?${query}` : "/", true);
}

/** The filters set in the panel, for the "Filters (n)" count. */
function panelFilters(filter: Filter, view: ViewId | "custom"): number {
  const owned = view === "custom" ? [] : Object.keys(VIEWS.find((v) => v.id === view)?.filter("") ?? {});
  return (
    [
      owned.includes("status") ? undefined : filter.status,
      filter.kind,
      filter.priority,
      owned.includes("assignee") ? undefined : filter.assignee,
      filter.label,
      filter.gateway,
      filter.channel,
      filter.source,
      filter.reporter,
      filter.from,
      filter.to,
      filter.has_record,
      filter.unseen,
    ] as unknown[]
  ).filter((v) => v !== undefined).length;
}

/** Whether a wide screen shows the drawer beside the list, else over it. */
function wide(): boolean {
  return typeof window !== "undefined" && typeof window.matchMedia === "function"
    ? window.matchMedia("(min-width: 1100px)").matches
    : true;
}

const linkOf = (id: string) => `${window.location.origin}/r/${id}`;

/** A report counts as unread while it is new and this admin has not opened it. */
const unread = (r: Summary) => r.status === "new" && !r.seen;

function Shortcuts({ lang, onClose }: { lang: Lang; onClose: () => void }) {
  const rows: [string, Key][] = [
    ["j / k, ↓ / ↑", "rep.key.move"],
    ["Enter", "rep.key.open"],
    ["o", "rep.key.page"],
    ["x", "rep.key.select"],
    ["a", "rep.key.selectAll"],
    ["1 – 6", "rep.key.status"],
    ["m", "rep.key.assign"],
    ["u", "rep.key.read"],
    ["#, Del", "rep.key.trash"],
    ["[ / ]", "rep.key.pages"],
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

/** A column the list can be sorted by, as a header that sorts it. */
function SortHeader({
  lang,
  sort,
  filter,
  label,
  set,
  className,
}: {
  lang: Lang;
  sort: Sort;
  filter: Filter;
  label: string;
  set: (change: FilterChange) => void;
  className?: string;
}) {
  const current = (filter.sort ?? "received") === sort;
  const dir = filter.dir ?? "desc";
  return (
    <th
      scope="col"
      className={className}
      aria-sort={current ? (dir === "asc" ? "ascending" : "descending") : undefined}
    >
      <button
        type="button"
        className="sort-head"
        title={t(lang, "rep.sortBy", { what: sortLabel(lang, sort) })}
        onClick={() => {
          if (current) set({ sort, dir: dir === "asc" ? "desc" : "asc" });
          else set({ sort, dir: firstDir(sort) });
        }}
      >
        {label}
        <span className="sort-mark" aria-hidden="true">
          {current ? (dir === "asc" ? "↑" : "↓") : ""}
        </span>
      </button>
    </th>
  );
}

function Pager({
  lang,
  filter,
  total,
}: {
  lang: Lang;
  filter: Filter;
  total: number;
}) {
  const size = filter.limit ?? PAGE_SIZE;
  const offset = filter.offset ?? 0;
  const page = Math.floor(offset / size) + 1;
  const pages = Math.max(1, Math.ceil(total / size));
  const to = (p: number) => {
    go({ ...filter, offset: (Math.min(Math.max(p, 1), pages) - 1) * size });
  };
  return (
    <nav className="pager" aria-label={t(lang, "rep.pages")}>
      <button type="button" disabled={page <= 1} onClick={() => { to(1); }}>
        « {t(lang, "rep.first")}
      </button>
      <button type="button" disabled={page <= 1} onClick={() => { to(page - 1); }}>
        ‹ {t(lang, "rep.prev")}
      </button>
      <span className="muted">{t(lang, "rep.page", { n: formatCount(lang, page), total: formatCount(lang, pages) })}</span>
      <button type="button" disabled={page >= pages} onClick={() => { to(page + 1); }}>
        {t(lang, "rep.next")} ›
      </button>
      <button type="button" disabled={page >= pages} onClick={() => { to(pages); }}>
        {t(lang, "rep.last")} »
      </button>
      <label htmlFor="page-size" className="muted small">
        {t(lang, "rep.pageSize")}
      </label>
      <select
        id="page-size"
        value={size}
        onChange={(event) => {
          const limit = Number(event.target.value);
          go({ ...filter, limit: limit === PAGE_SIZE ? undefined : limit, offset: undefined });
        }}
      >
        {PAGE_SIZES.map((s) => (
          <option key={s} value={s}>
            {s}
          </option>
        ))}
      </select>
    </nav>
  );
}

export function ReportList({ search, lang = "en", me = "" }: { search: string; lang?: Lang; me?: string }) {
  const filter = useMemo(() => parseFilter(search), [search]);
  const [listing, setListing] = useState<Listing | null>(null);
  const [arrived, setArrived] = useState<Listing | null>(null);
  const [facets, setFacets] = useState<Facets | null>(null);
  const [counts, setCounts] = useState<Partial<Record<ViewId, number>>>({});
  const [admins, setAdmins] = useState<string[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [cursor, setCursor] = useState(0);
  const [checked, setChecked] = useState<ReadonlySet<string>>(new Set());
  const [drawer, setDrawer] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [updatedAt, setUpdatedAt] = useState<number | null>(null);
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [help, setHelp] = useState(false);
  const [busy, setBusy] = useState(false);
  const [groups, setGroups] = useState(false);
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

  /** The views' counts: each over its own filter alone. */
  const loadCounts = useCallback(() => {
    const total = (f: Filter) => api.facets(f).then((x) => x.total);
    Promise.all([
      api.facets({}),
      me === "" ? Promise.resolve(undefined) : total({ status: [...OPEN_STATUSES], assignee: me }),
      total({ status: [...OPEN_STATUSES], assignee: "none" }),
      total({ trash: true }),
    ])
      .then(([all, mine, unassigned, trash]) => {
        const of = (s: string) => all.statuses.find((c) => c.value === s)?.count ?? 0;
        setCounts({
          inbox: of("new"),
          open: OPEN_STATUSES.reduce((sum, s) => sum + of(s), 0),
          ...(mine === undefined ? {} : { mine }),
          unassigned,
          done: DONE_STATUSES.reduce((sum, s) => sum + of(s), 0),
          all: all.total,
          trash,
        });
      })
      .catch(() => {
        // The views work without their counts.
      });
  }, [me]);

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
      api
        .facets(filter)
        .then(setFacets)
        .catch(() => {
          // The filters work without their counts.
        });
      loadCounts();
    },
    [filter, describe, loadCounts],
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
      .admins()
      .then(setAdmins)
      .catch(() => {
        // Assigning offers the signed-in admin alone.
      });
  }, []);

  /** The Refresh button and `r`: asked at once, with the spinner on. */
  const refresh = () => {
    setRefreshing(true);
    load(false);
  };

  const reports = useMemo(() => listing?.reports ?? [], [listing]);
  const under = reports[cursor];
  const view = viewOf(filter, me);
  const inTrash = filter.trash === true;

  const showArrived = () => {
    if (arrived === null) return;
    setListing(arrived);
    setArrived(null);
    setCursor(0);
  };

  /** Rows as this admin now has them opened or not. */
  const markRows = useCallback((ids: readonly string[], seen: boolean) => {
    const set = new Set(ids);
    setListing((was) =>
      was === null ? was : { ...was, reports: was.reports.map((r) => (set.has(r.id) ? { ...r, seen } : r)) },
    );
  }, []);

  const markSeen = useCallback(
    (ids: readonly string[], seen: boolean) => {
      if (ids.length === 0) return;
      markRows(ids, seen);
      api.seen(ids, seen).catch(() => {
        // The marker is a convenience; the next refresh says how it is.
      });
    },
    [markRows],
  );

  const openDrawer = useCallback(
    (id: string) => {
      markSeen([id], true);
      setDrawer(id);
    },
    [markSeen],
  );

  const openPage = (id: string) => {
    markSeen([id], true);
    navigate(`/r/${id}`);
  };

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

  /** The reports an action works on: the checked ones, else the one under the cursor. */
  const targets = (): Summary[] =>
    checked.size > 0 ? reports.filter((r) => checked.has(r.id)) : under ? [under] : [];

  const exportSelected = () => {
    const chosen = targets().length > 1 ? targets() : reports;
    if (chosen.length === 0) return;
    const day = new Date().toISOString().slice(0, 10);
    download(`baylee-reports-${day}.csv`, toCsv(chosen));
    toast("ok", t(lang, "rep.exported", { n: chosen.length }));
  };

  /** One change for `ids` (the targets), in one request: all or none. */
  const bulkChange = async (change: BulkChange, ids: readonly string[] = targets().map((r) => r.id)) => {
    if (ids.length === 0 || busy) return;
    setBusy(true);
    try {
      const done = await api.bulk(ids, { change });
      toast("ok", t(lang, "rep.changedN", { n: done.changed }));
      load(false);
    } catch (e: unknown) {
      if (!(e instanceof SignedOut)) toast("error", t(lang, "rep.bulkRefused", { why: describe(e) }));
    } finally {
      setBusy(false);
    }
  };

  /** Into the trash (with an undo), out of it, or gone for good. */
  const bulkAction = async (action: BulkAction, ids: readonly string[] = targets().map((r) => r.id)) => {
    if (ids.length === 0 || busy) return;
    setBusy(true);
    try {
      const done = await api.bulk(ids, { action });
      if (action === "trash") {
        toast("ok", t(lang, "rep.trashed", { n: done.changed }), {
          label: t(lang, "rep.undo"),
          run: () => {
            api
              .bulk(ids, { action: "restore" })
              .then((back) => {
                toast("ok", t(lang, "rep.restored", { n: back.changed }));
                load(false);
              })
              .catch((e: unknown) => {
                if (!(e instanceof SignedOut)) toast("error", describe(e));
              });
          },
        });
      } else {
        toast("ok", t(lang, action === "restore" ? "rep.restored" : "rep.purged", { n: done.changed }));
      }
      setChecked(new Set());
      if (drawer !== null && ids.includes(drawer)) setDrawer(null);
      load(false);
    } catch (e: unknown) {
      if (!(e instanceof SignedOut)) toast("error", t(lang, "rep.bulkRefused", { why: describe(e) }));
    } finally {
      setBusy(false);
    }
  };

  // Any change of filter starts again at the first page.
  const set = (change: FilterChange) => {
    go(applyChange(filter, change));
  };

  const offset = filter.offset ?? 0;
  const size = filter.limit ?? PAGE_SIZE;
  const total = listing?.total ?? 0;

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
        exportSelected();
      } else if (key === "?") {
        stop();
        setHelp((h) => !h);
      } else if (key === "m" && !inTrash && targets().length > 0 && me !== "") {
        stop();
        const mine = targets().every((r) => r.assignee === me);
        void bulkChange({ assignee: mine ? null : me });
      } else if (key === "u" && under) {
        stop();
        markSeen([under.id], !under.seen);
      } else if ((key === "#" || key === "Delete") && !inTrash && targets().length > 0) {
        stop();
        void bulkAction("trash");
      } else if (key === "[" && offset > 0) {
        stop();
        go({ ...filter, offset: Math.max(offset - size, 0) });
      } else if (key === "]" && offset + size < total) {
        stop();
        go({ ...filter, offset: offset + size });
      } else if (/^[1-6]$/.test(key) && !inTrash) {
        const status = STATUSES[Number(key) - 1];
        if (status) {
          stop();
          void bulkChange({ status });
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

  // The search box is applied on Enter or when it loses focus, not per key.
  const applySearch = () => {
    const q = searchBox.current?.value.trim() ?? "";
    if (q !== (filter.q ?? "")) set({ q: q === "" ? undefined : q });
  };

  const submitSearch = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    applySearch();
  };

  const owned = new Set<keyof Filter>(
    view === "custom" ? [] : (Object.keys(VIEWS.find((v) => v.id === view)?.filter(me) ?? {}) as (keyof Filter)[]),
  );
  const pills = pillsOf(lang, filter, owned);
  const extras = panelFilters(filter, view);
  const allChecked = reports.length > 0 && reports.every((r) => checked.has(r.id));
  const unreadHere = reports.filter(unread).length;
  const n = (v: number) => formatCount(lang, v);
  const selected = reports.filter((r) => checked.has(r.id));
  const labels = (facets?.labels ?? []).flatMap((c) => (c.value === null ? [] : [c.value]));

  const onRowClick = (event: MouseEvent<HTMLTableRowElement>, report: Summary, index: number) => {
    const target = event.target;
    if (target instanceof HTMLElement && target.closest("a, button, input, select, label")) return;
    setCursor(index);
    openDrawer(report.id);
  };

  return (
    <main className={`list-page${drawer !== null ? " with-drawer" : ""}`} id="main">
      <div className="list-main">
        <h1 className="sr-only">{t(lang, "nav.reports")}</h1>
        <div className="list-toolbar">
          <fieldset className="views segmented">
            <legend className="sr-only">{t(lang, "rep.views")}</legend>
            {VIEWS.filter((v) => v.id !== "mine" || me !== "").map((v) => (
              <button
                key={v.id}
                type="button"
                aria-pressed={view === v.id}
                onClick={() => {
                  const f = v.filter(me);
                  set({ status: f.status, assignee: f.assignee, trash: f.trash });
                }}
              >
                {t(lang, v.label)}
                {counts[v.id] !== undefined && v.id !== "all" && v.id !== "done" && (
                  <span className="seg-count">{n(counts[v.id] ?? 0)}</span>
                )}
              </button>
            ))}
          </fieldset>
          <SavedViews lang={lang} query={filterQuery({ ...filter, offset: undefined })} />
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
            aria-expanded={filtersOpen}
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
                time: new Intl.DateTimeFormat(localeOf(lang), { timeStyle: "short" }).format(
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

        <FilterPanel
          lang={lang}
          filter={filter}
          facets={facets}
          admins={admins}
          me={me}
          set={set}
          onSubmit={applySearch}
          hidden={!filtersOpen}
        />

        <Pills
          lang={lang}
          pills={pills}
          set={set}
          onClear={() => {
            const f = view === "custom" ? {} : (VIEWS.find((v) => v.id === view)?.filter(me) ?? {});
            go({ ...f, sort: filter.sort, dir: filter.dir, limit: filter.limit });
          }}
        />

        {error !== null && (
          <p className="error" role="alert">
            {error}
          </p>
        )}

        {inTrash && <p className="muted small trash-hint">{t(lang, "rep.trashHint", { days: TRASH_DAYS })}</p>}

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

        {checked.size > 0 && !groups ? (
          <BulkBar
            lang={lang}
            selected={selected}
            me={me}
            admins={admins}
            labels={labels}
            inTrash={inTrash}
            busy={busy}
            onChange={(change, ids) => void bulkChange(change, ids)}
            onAction={(action) => void bulkAction(action)}
            onSeen={(seen) => {
              markSeen([...checked], seen);
            }}
            onCopyIds={() => void copyText([...checked].join("\n"))}
            onExport={exportSelected}
            onDeselect={() => {
              setChecked(new Set());
            }}
          />
        ) : (
          <div className="count-line">
            <p className="count muted" aria-live="polite">
              {listing === null
                ? t(lang, "rep.loading")
                : total === 0
                  ? t(lang, "rep.none")
                  : t(lang, "rep.range", { from: n(offset + 1), to: n(offset + reports.length), total: n(total) })}
              {unreadHere > 0 && (
                <span className="unread-count">
                  {" "}
                  · {unreadHere} {t(lang, "rep.unread")}
                </span>
              )}
              {total > 0 && (
                <>
                  {" "}
                  <a className="link small" href={api.csvUrl(filter)} download>
                    {t(lang, "rep.exportAll", { n: n(Math.min(total, 5000)) })}
                  </a>
                </>
              )}
            </p>
            {!inTrash && (facets?.kinds.some((k) => k.value === "crash") ?? false) && (
              <fieldset className="segmented small-seg">
                <legend className="sr-only">{t(lang, "rep.shownAs")}</legend>
                <button
                  type="button"
                  aria-pressed={!groups}
                  onClick={() => {
                    setGroups(false);
                  }}
                >
                  {t(lang, "rep.listMode")}
                </button>
                <button
                  type="button"
                  aria-pressed={groups}
                  onClick={() => {
                    setGroups(true);
                  }}
                >
                  {t(lang, "rep.groups")}
                </button>
              </fieldset>
            )}
          </div>
        )}

        {groups ? (
          <CrashGroups
            lang={lang}
            filter={filter}
            onPick={(signature) => {
              setGroups(false);
              set({ signature });
            }}
          />
        ) : (
          reports.length > 0 && (
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
                    <SortHeader lang={lang} sort="received" filter={filter} set={set} label={t(lang, "rep.col.received")} />
                    <SortHeader lang={lang} sort="kind" filter={filter} set={set} label={t(lang, "rep.col.kind")} />
                    <SortHeader lang={lang} sort="status" filter={filter} set={set} label={t(lang, "rep.col.status")} />
                    <SortHeader lang={lang} sort="priority" filter={filter} set={set} label={t(lang, "rep.col.priority")} />
                    <th scope="col" className="wide">
                      {t(lang, "rep.col.text")}
                    </th>
                    <th scope="col">{t(lang, "rep.col.assignee")}</th>
                    <SortHeader lang={lang} sort="gateway" filter={filter} set={set} label={t(lang, "rep.col.gateway")} />
                    <SortHeader lang={lang} sort="reporter" filter={filter} set={set} label={t(lang, "rep.col.reporter")} />
                    <th scope="col">{t(lang, "rep.col.record")}</th>
                    <th scope="col">{t(lang, "rep.col.issue")}</th>
                  </tr>
                </thead>
                <tbody>
                  {reports.map((report, index) => {
                    const isUnread = unread(report);
                    const classes = [
                      index === cursor ? "selected" : "",
                      isUnread ? "unread" : "",
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
                          {isUnread && <span className="unread-dot" title={t(lang, "rep.unread")} />}
                          <Time lang={lang} iso={report.created_at} />
                        </td>
                        <td>
                          <span className={`tag kind-${report.kind}`}>{kindLabel(lang, report.kind)}</span>
                        </td>
                        <td>
                          <span className={`tag status-${report.status}`}>{statusLabel(lang, report.status)}</span>
                        </td>
                        <td>
                          <PriorityTag lang={lang} priority={report.priority} />
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
                          <span className="row-marks">
                            <Labels
                              labels={report.labels}
                              onPick={(label) => {
                                set({ label });
                              }}
                            />
                            {report.notes > 0 && (
                              <span className="note-count" title={t(lang, "rep.notesN", { n: report.notes })}>
                                ✎ {report.notes}
                              </span>
                            )}
                            {report.duplicate_of !== null && (
                              <span className="muted small">
                                {t(lang, "rep.dupBadge", { id: shortId(report.duplicate_of) })}
                              </span>
                            )}
                          </span>
                        </td>
                        <td>
                          {report.assignee === null ? (
                            <span className="muted">—</span>
                          ) : (
                            <button
                              type="button"
                              className="link"
                              onClick={() => {
                                set({ assignee: report.assignee ?? undefined });
                              }}
                            >
                              {report.assignee}
                            </button>
                          )}
                        </td>
                        <td>
                          {report.channel === "direct" ? "direct" : report.gateway}
                          {report.source !== null && (
                            <>
                              {" "}
                              {isSourceName(report.source) ? (
                                <button
                                  type="button"
                                  className="tag link"
                                  title={t(lang, "rep.sourceTitle")}
                                  onClick={() => {
                                    set({ source: report.source ?? undefined });
                                  }}
                                >
                                  {report.source}
                                </button>
                              ) : (
                                <span className="tag" title={t(lang, "rep.sourceTitle")}>
                                  {report.source}
                                </span>
                              )}
                            </>
                          )}
                        </td>
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
          )
        )}

        {!groups && total > size && <Pager lang={lang} filter={filter} total={total} />}
      </div>

      {drawer !== null && (
        <Drawer
          key={drawer}
          id={drawer}
          lang={lang}
          me={me}
          onClose={() => {
            setDrawer(null);
          }}
          onChanged={(updated) => {
            // A report the list no longer has (restored from the trash by
            // the drawer's undo) comes back by asking the list again.
            if (current.current !== null && !current.current.reports.some((r) => r.id === updated.id)) {
              load(false);
              return;
            }
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
