import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";

import {
  api,
  filterQuery,
  KINDS,
  PAGE_SIZE,
  parseFilter,
  Refused,
  SignedOut,
  STATUS_LABELS,
  STATUSES,
  type Facets,
  type Filter,
  type Listing,
} from "./api";
import { firstLine } from "./github";
import { formatBytes, formatTime, shortPseudonym } from "./format";
import { navigate } from "./router";

/** A change to the filters; `undefined` clears one. */
type FilterChange = { [K in keyof Filter]?: Filter[K] | undefined };

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

export function ReportList({ search }: { search: string }) {
  const filter = useMemo(() => parseFilter(search), [search]);
  const [listing, setListing] = useState<Listing | null>(null);
  const [facets, setFacets] = useState<Facets | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState(0);
  const searchBox = useRef<HTMLInputElement>(null);
  const rows = useRef<(HTMLTableRowElement | null)[]>([]);

  useEffect(() => {
    let live = true;
    api
      .reports(filter)
      .then((l) => {
        if (!live) return;
        setError(null);
        setListing(l);
        setSelected(0);
      })
      .catch((e: unknown) => {
        if (live && !(e instanceof SignedOut))
          setError(e instanceof Refused ? e.message : "The service did not answer.");
      });
    return () => {
      live = false;
    };
  }, [filter]);

  useEffect(() => {
    api
      .facets()
      .then(setFacets)
      .catch(() => {
        // The filters work without their suggestions.
      });
  }, []);

  const reports = useMemo(() => listing?.reports ?? [], [listing]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey || typing(event.target)) return;
      if (event.key === "/") {
        event.preventDefault();
        searchBox.current?.focus();
      } else if ((event.key === "j" || event.key === "ArrowDown") && reports.length > 0) {
        event.preventDefault();
        setSelected((s) => Math.min(s + 1, reports.length - 1));
      } else if ((event.key === "k" || event.key === "ArrowUp") && reports.length > 0) {
        event.preventDefault();
        setSelected((s) => Math.max(s - 1, 0));
      } else if (event.key === "Enter") {
        const report = reports[selected];
        if (report) {
          event.preventDefault();
          navigate(`/r/${report.id}`);
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, [reports, selected]);

  useEffect(() => {
    rows.current[selected]?.scrollIntoView?.({ block: "nearest" });
  }, [selected]);

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

  return (
    <main className="list-page">
      <form className="filters" onSubmit={submitSearch} aria-label="Filter reports">
        <div className="field grow">
          <label htmlFor="f-q">Text</label>
          <input
            id="f-q"
            ref={searchBox}
            type="search"
            placeholder="Search the text  ( / )"
            key={filter.q ?? ""}
            defaultValue={filter.q ?? ""}
            onBlur={applySearch}
          />
        </div>
        <div className="field">
          <label htmlFor="f-kind">Kind</label>
          <select
            id="f-kind"
            value={filter.kind ?? ""}
            onChange={(event) => {
              set({ kind: KINDS.find((k) => k === event.target.value) });
            }}
          >
            <option value="">Any</option>
            {KINDS.map((kind) => (
              <option key={kind} value={kind}>
                {kind}
              </option>
            ))}
          </select>
        </div>
        <div className="field">
          <label htmlFor="f-status">Status</label>
          <select
            id="f-status"
            value={filter.status ?? ""}
            onChange={(event) => {
              set({ status: STATUSES.find((s) => s === event.target.value) });
            }}
          >
            <option value="">Any</option>
            {STATUSES.map((status) => (
              <option key={status} value={status}>
                {STATUS_LABELS[status]}
              </option>
            ))}
          </select>
        </div>
        <div className="field">
          <label htmlFor="f-gateway">Gateway</label>
          <select
            id="f-gateway"
            value={filter.gateway ?? ""}
            onChange={(event) => {
              set({ gateway: event.target.value || undefined });
            }}
          >
            <option value="">Any</option>
            {(facets?.gateways ?? []).map((g) => (
              <option key={g.value} value={g.value}>
                {g.value} ({g.count})
              </option>
            ))}
            {filter.gateway && !facets?.gateways.some((g) => g.value === filter.gateway) && (
              <option value={filter.gateway}>{filter.gateway}</option>
            )}
          </select>
        </div>
        <div className="field">
          <label htmlFor="f-reporter">Reporter</label>
          <input
            id="f-reporter"
            list="f-reporters"
            spellCheck={false}
            placeholder="Pseudonym"
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
                {r.count} report(s)
              </option>
            ))}
          </datalist>
        </div>
        <div className="field">
          <label htmlFor="f-from">From</label>
          <input
            id="f-from"
            type="date"
            value={filter.from ?? ""}
            onChange={(event) => {
              set({ from: event.target.value || undefined });
            }}
          />
        </div>
        <div className="field">
          <label htmlFor="f-to">To</label>
          <input
            id="f-to"
            type="date"
            value={filter.to ?? ""}
            onChange={(event) => {
              set({ to: event.target.value || undefined });
            }}
          />
        </div>
        <div className="field">
          <label htmlFor="f-record">Game record</label>
          <select
            id="f-record"
            value={filter.has_record === undefined ? "" : String(filter.has_record)}
            onChange={(event) => {
              const v = event.target.value;
              set({ has_record: v === "" ? undefined : v === "true" });
            }}
          >
            <option value="">Any</option>
            <option value="true">With record</option>
            <option value="false">Without</option>
          </select>
        </div>
        <div className="field actions">
          <button type="submit" className="primary">
            Search
          </button>
          {anyFilter && (
            <button
              type="button"
              className="quiet"
              onClick={() => {
                go({});
              }}
            >
              Clear
            </button>
          )}
        </div>
      </form>

      {facets && facets.reporters.length > 0 && (
        <details className="reporters">
          <summary>Reporters by number of reports</summary>
          <ul>
            {facets.reporters.map((r) => (
              <li key={r.value}>
                <button
                  type="button"
                  className="link mono"
                  title={r.value}
                  onClick={() => {
                    set({ reporter: r.value });
                  }}
                >
                  {shortPseudonym(r.value)}
                </button>{" "}
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

      <p className="count muted" aria-live="polite">
        {listing === null
          ? "Loading…"
          : total === 0
            ? "No reports match."
            : `${offset + 1}–${offset + reports.length} of ${total}`}
        <span className="hint"> · j/k to move, Enter to open, / to search</span>
      </p>

      {reports.length > 0 && (
        <div className="table-wrap">
          <table className="reports">
            <thead>
              <tr>
                <th scope="col">Received</th>
                <th scope="col">Kind</th>
                <th scope="col">Status</th>
                <th scope="col" className="wide">
                  Text
                </th>
                <th scope="col">Gateway</th>
                <th scope="col">Reporter</th>
                <th scope="col">Record</th>
                <th scope="col">Issue</th>
              </tr>
            </thead>
            <tbody>
              {reports.map((report, index) => (
                <tr
                  key={report.id}
                  ref={(el) => {
                    rows.current[index] = el;
                  }}
                  className={index === selected ? "selected" : undefined}
                  aria-selected={index === selected}
                  onClick={() => {
                    setSelected(index);
                  }}
                >
                  <td className="nowrap">{formatTime(report.created_at)}</td>
                  <td>
                    <span className={`tag kind-${report.kind}`}>{report.kind}</span>
                  </td>
                  <td>
                    <span className={`tag status-${report.status}`}>{STATUS_LABELS[report.status]}</span>
                  </td>
                  <td className="wide">
                    <a
                      href={`/r/${report.id}`}
                      onClick={(event) => {
                        event.preventDefault();
                        navigate(`/r/${report.id}`);
                      }}
                    >
                      {firstLine(report.text, 120) || <span className="muted">(no text)</span>}
                    </a>
                  </td>
                  <td>{report.channel === "direct" ? "direct" : report.gateway}</td>
                  <td>
                    <button
                      type="button"
                      className="link mono"
                      title={`Only reports by ${report.reporter}`}
                      onClick={(event) => {
                        event.stopPropagation();
                        set({ reporter: report.reporter });
                      }}
                    >
                      {shortPseudonym(report.reporter)}
                    </button>
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
              ))}
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
    </main>
  );
}
