// The pool's progress (`/admin/sets`, `/ui/api/admin/sets`): how far this
// build is through each set, in release order, from the gateway's own
// ledger and compiled pool; one set (`/admin/sets/{code}`) lists its cards
// with how far each is. A set counts the cards first printed in it. A
// card's picture, on hover, comes from Scryfall into the admin's browser
// (docs/legal.md §3), never through the service.

import { useCallback, useId, useState, type ReactNode } from "react";

import {
  api,
  COVERAGES,
  SignedOut,
  type Coverage,
  type SetCard,
  type SetDetail,
  type SetProgress,
  type SetsOverview,
} from "../api";
import { formatCount, t, type Key, type Lang } from "../i18n";
import { CardChip } from "../reports/pieces";
import { setUrl } from "../reports/scryfall";
import { adminPath } from "../router";
import {
  Badge,
  describe,
  Freshness,
  Link,
  Panel,
  useVisibleInterval,
  type Tone,
} from "./shared";

const LABEL: Record<Coverage, Key> = {
  implemented: "sets.implemented",
  partial: "sets.partial",
  unimplemented: "sets.unimplemented",
  absent: "sets.absent",
};

const TONE: Record<Coverage, Tone> = {
  implemented: "ok",
  partial: "warn",
  unimplemented: "info",
  absent: "muted",
};

/** How often the progress is asked again: it changes with a deploy only. */
const SETS_MS = 120_000;

function share(part: number, total: number): number {
  return total === 0 ? 0 : part / total;
}

/** A stacked bar of the four buckets, the same words as its legend. */
export function ProgressBar({
  lang,
  counts,
  label,
}: {
  lang: Lang;
  counts: SetProgress;
  label: string;
}) {
  const pct = (n: number) => `${(share(n, counts.total) * 100).toFixed(2)}%`;
  const text = COVERAGES.map(
    (c) => `${t(lang, LABEL[c])} ${formatCount(lang, counts[c])}`,
  ).join(", ");
  return (
    <div className="progress-wrap">
      <span className="sr-only">{`${label}: ${text}`}</span>
      <div className="progress" aria-hidden="true">
        {COVERAGES.map((c) =>
          counts[c] === 0 ? null : (
            <span
              key={c}
              className={`progress-${c}`}
              style={{ width: pct(counts[c]) }}
            />
          ),
        )}
      </div>
    </div>
  );
}

/** The headline: implemented of total, as a number and a bar. */
export function PoolSummary({
  lang,
  overview,
  compact = false,
}: {
  lang: Lang;
  overview: SetsOverview;
  compact?: boolean;
}) {
  const n = (v: number) => formatCount(lang, v);
  const whole: SetProgress = {
    code: "",
    position: 0,
    total: overview.corpus,
    ...overview.pool,
  };
  const done = share(overview.pool.implemented, overview.corpus);
  return (
    <div className="pool-summary">
      <p className="big" data-testid="pool-implemented">
        <span className="big-number">{n(overview.pool.implemented)}</span>{" "}
        <span className="big-label">
          {t(lang, "sets.ofCorpus", {
            total: n(overview.corpus),
            pct: (done * 100).toFixed(1),
          })}
        </span>
      </p>
      <ProgressBar
        lang={lang}
        counts={whole}
        label={t(lang, "sets.wholePool")}
      />
      {!compact && (
        <ul className="legend-list">
          {COVERAGES.map((c) => (
            <li key={c}>
              <span className={`swatch progress-${c}`} aria-hidden="true" />
              {t(lang, LABEL[c])} <b>{n(overview.pool[c])}</b>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function useSets(
  lang: Lang,
): [SetsOverview | null, string | null, boolean, () => void] {
  const [overview, setOverview] = useState<SetsOverview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(() => {
    api.admin
      .sets()
      .then((o) => {
        setOverview(o);
        setError(null);
      })
      .catch((e: unknown) => {
        if (!(e instanceof SignedOut)) setError(describe(lang, e));
      });
  }, [lang]);
  const shown = useVisibleInterval(load, SETS_MS);
  return [overview, error, shown, load];
}

/** The pool on the Live page: the headline and the sets being worked on. */
export function PoolPanel({ lang }: { lang: Lang }) {
  const [overview, error] = useSets(lang);
  const n = (v: number) => formatCount(lang, v);
  // The first sets that are not complete: where the work is.
  const open = (overview?.sets ?? [])
    .filter((s) => s.implemented < s.total)
    .slice(0, 5);
  return (
    <Panel
      id="live-pool"
      title={t(lang, "sets.title")}
      className="live-pool"
      actions={
        <Link to={adminPath("sets")} className="button small-button">
          {t(lang, "sets.all")} →
        </Link>
      }
    >
      {error !== null && <p className="error">{error}</p>}
      {overview !== null && (
        <>
          <PoolSummary lang={lang} overview={overview} compact />
          <ul className="set-rows">
            {open.map((s) => (
              <li key={s.code} className="set-row">
                <Link to={adminPath("sets", s.code)} className="set-code">
                  {s.code.toUpperCase()}
                </Link>
                <span className="muted small">
                  {n(s.implemented)} / {n(s.total)}
                </span>
                <ProgressBar
                  lang={lang}
                  counts={s}
                  label={s.code.toUpperCase()}
                />
              </li>
            ))}
          </ul>
        </>
      )}
    </Panel>
  );
}

type Sort = "release" | "size" | "done" | "open";

function sorted(sets: SetProgress[], sort: Sort): SetProgress[] {
  const list = [...sets];
  switch (sort) {
    case "size":
      return list.sort((a, b) => b.total - a.total || a.position - b.position);
    case "done":
      return list.sort(
        (a, b) =>
          share(b.implemented, b.total) - share(a.implemented, a.total) ||
          a.position - b.position,
      );
    case "open":
      return list.sort(
        (a, b) =>
          b.total - b.implemented - (a.total - a.implemented) ||
          a.position - b.position,
      );
    default:
      return list;
  }
}

export function Sets({ lang }: { lang: Lang }) {
  const [overview, error, shown, load] = useSets(lang);
  const [needle, setNeedle] = useState("");
  const [sort, setSort] = useState<Sort>("release");
  const [onlyOpen, setOnlyOpen] = useState(false);
  const n = (v: number) => formatCount(lang, v);
  const q = needle.trim().toLowerCase();
  const rows = sorted(
    (overview?.sets ?? []).filter(
      (s) =>
        (q === "" || s.code.includes(q)) &&
        (!onlyOpen || s.implemented < s.total),
    ),
    sort,
  );
  return (
    <>
      <div className="page-head">
        <div>
          <h1>{t(lang, "sets.title")}</h1>
          <Freshness lang={lang} at={overview?.at ?? null} shown={shown} />
        </div>
        <button type="button" onClick={load}>
          {t(lang, "admin.refresh")}
        </button>
      </div>
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {overview !== null && (
        <>
          <Panel
            id="sets-pool"
            title={t(lang, "sets.wholePool")}
            className="wide-panel"
          >
            <PoolSummary lang={lang} overview={overview} />
            <p className="muted small">
              {t(lang, "sets.howCounted")} · {t(lang, "stats.version")}:{" "}
              {overview.version}
            </p>
          </Panel>
          <form
            className="toolbar"
            onSubmit={(event) => {
              event.preventDefault();
            }}
          >
            <input
              type="search"
              className="grow"
              aria-label={t(lang, "sets.search")}
              placeholder={t(lang, "sets.search")}
              value={needle}
              onChange={(event) => {
                setNeedle(event.target.value);
              }}
            />
            <label className="select-label">
              {t(lang, "sets.sort")}
              <select
                value={sort}
                onChange={(event) => {
                  setSort(event.target.value as Sort);
                }}
              >
                <option value="release">{t(lang, "sets.sortRelease")}</option>
                <option value="size">{t(lang, "sets.sortSize")}</option>
                <option value="done">{t(lang, "sets.sortDone")}</option>
                <option value="open">{t(lang, "sets.sortOpen")}</option>
              </select>
            </label>
            <button
              type="button"
              className="toggle"
              aria-pressed={onlyOpen}
              onClick={() => {
                setOnlyOpen(!onlyOpen);
              }}
            >
              <span className="presence-dot" aria-hidden="true" />
              {t(lang, "sets.onlyOpen")}
            </button>
          </form>
          <p className="muted small" aria-live="polite">
            {t(lang, "sets.count", {
              n: n(rows.length),
              total: n(overview.sets.length),
            })}
          </p>
          <div className="sets-scroll">
            <table className="sets">
              <thead>
                <tr>
                  <th scope="col">#</th>
                  <th scope="col">{t(lang, "sets.set")}</th>
                  <th scope="col" className="num">
                    {t(lang, "sets.cards")}
                  </th>
                  <th scope="col" className="num">
                    {t(lang, "sets.implemented")}
                  </th>
                  <th scope="col" className="num">
                    {t(lang, "sets.partial")}
                  </th>
                  <th scope="col" className="num">
                    {t(lang, "sets.unimplemented")}
                  </th>
                  <th scope="col" className="num">
                    {t(lang, "sets.absent")}
                  </th>
                  <th scope="col" className="set-bar">
                    {t(lang, "sets.progress")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {rows.map((s) => (
                  <tr key={s.code} data-testid="set">
                    <td className="num muted set-no" data-label="#">
                      {s.position + 1}
                    </td>
                    <td className="set-name" data-label={t(lang, "sets.set")}>
                      <Link to={adminPath("sets", s.code)} className="set-code">
                        {s.code.toUpperCase()}
                      </Link>
                    </td>
                    <td className="num" data-label={t(lang, "sets.cards")}>
                      {n(s.total)}
                    </td>
                    <td className="num" data-label={t(lang, "sets.implemented")}>
                      {n(s.implemented)}
                    </td>
                    <td className="num" data-label={t(lang, "sets.partial")}>
                      {n(s.partial)}
                    </td>
                    <td className="num" data-label={t(lang, "sets.unimplemented")}>
                      {n(s.unimplemented)}
                    </td>
                    <td className="num" data-label={t(lang, "sets.absent")}>
                      {n(s.absent)}
                    </td>
                    <td className="set-bar" data-label={t(lang, "sets.progress")}>
                      <ProgressBar
                        lang={lang}
                        counts={s}
                        label={s.code.toUpperCase()}
                      />
                      <span className="muted small">
                        {(share(s.implemented, s.total) * 100).toFixed(0)} %
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </>
  );
}

/** One card's row: its chip (a picture on hover), its type, and how far it is. */
function CardRow({ lang, card }: { lang: Lang; card: SetCard }) {
  return (
    <li className="set-card" data-testid="set-card">
      <span className="set-card-name">
        <CardChip name={card.name} scryfallId={card.scryfall_id} lang={lang} />
      </span>
      <span className="muted small set-card-type">{card.type_line}</span>
      <Badge tone={TONE[card.coverage]}>{t(lang, LABEL[card.coverage])}</Badge>
      {card.note !== null && (
        <span className="muted small set-card-note">{card.note}</span>
      )}
    </li>
  );
}

export function SetPage({ lang, code }: { lang: Lang; code: string }) {
  const [set, setSet] = useState<SetDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<Coverage | "all">("all");
  const [needle, setNeedle] = useState("");
  const id = useId();
  const load = useCallback(() => {
    api.admin
      .set(code)
      .then((s) => {
        setSet(s);
        setError(null);
      })
      .catch((e: unknown) => {
        if (!(e instanceof SignedOut)) setError(describe(lang, e));
      });
  }, [code, lang]);
  useVisibleInterval(load, 0);
  const n = (v: number) => formatCount(lang, v);
  const q = needle.trim().toLowerCase();
  const cards = (set?.cards ?? []).filter(
    (c) =>
      (filter === "all" || c.coverage === filter) &&
      (q === "" || c.name.toLowerCase().includes(q)),
  );
  const page = setUrl(code);
  let body: ReactNode;
  if (error !== null) {
    body = (
      <p className="error" role="alert">
        {error}
      </p>
    );
  } else if (set === null) {
    body = <div className="skeleton" aria-hidden="true" />;
  } else {
    body = (
      <>
        <Panel
          id={`${id}-sum`}
          title={t(lang, "sets.progress")}
          className="wide-panel"
        >
          <p className="big">
            <span className="big-number">{n(set.implemented)}</span>{" "}
            <span className="big-label">
              {t(lang, "sets.ofCorpus", {
                total: n(set.total),
                pct: (share(set.implemented, set.total) * 100).toFixed(1),
              })}
            </span>
          </p>
          <ProgressBar lang={lang} counts={set} label={code.toUpperCase()} />
          <fieldset className="segmented chips">
            <legend className="sr-only">{t(lang, "sets.filter")}</legend>
            <button
              type="button"
              aria-pressed={filter === "all"}
              onClick={() => {
                setFilter("all");
              }}
            >
              {t(lang, "sets.allCards")}{" "}
              <span className="seg-count">{n(set.total)}</span>
            </button>
            {COVERAGES.map((c) => (
              <button
                key={c}
                type="button"
                aria-pressed={filter === c}
                onClick={() => {
                  setFilter(c);
                }}
              >
                {t(lang, LABEL[c])}{" "}
                <span className="seg-count">{n(set[c])}</span>
              </button>
            ))}
          </fieldset>
        </Panel>
        <form
          className="toolbar"
          onSubmit={(event) => {
            event.preventDefault();
          }}
        >
          <input
            type="search"
            className="grow"
            aria-label={t(lang, "sets.searchCards")}
            placeholder={t(lang, "sets.searchCards")}
            value={needle}
            onChange={(event) => {
              setNeedle(event.target.value);
            }}
          />
        </form>
        <p className="muted small" aria-live="polite">
          {t(lang, "sets.cardCount", {
            n: n(cards.length),
            total: n(set.total),
          })}
        </p>
        {cards.length === 0 ? (
          <p className="empty">{t(lang, "sets.noCards")}</p>
        ) : (
          <ul className="set-cards">
            {cards.map((card) => (
              <CardRow key={card.index} lang={lang} card={card} />
            ))}
          </ul>
        )}
      </>
    );
  }
  return (
    <>
      <p className="crumbs">
        <Link to={adminPath("sets")}>← {t(lang, "sets.title")}</Link>
      </p>
      <div className="page-head">
        <div>
          <h1>{code.toUpperCase()}</h1>
          <p className="muted small">
            {set !== null &&
              `${t(lang, "sets.position", { n: n(set.position + 1) })} · `}
            {page !== null && (
              <a href={page} target="_blank" rel="noopener noreferrer">
                {t(lang, "sets.onScryfall")}
              </a>
            )}
          </p>
        </div>
      </div>
      {body}
    </>
  );
}
