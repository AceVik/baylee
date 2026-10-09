// The accounts (`/admin/accounts`): searched, filtered and sorted in the
// query string, so a view can be linked and survives a reload; a table on
// a wide screen, a list of cards on a phone.

import { useEffect, useRef, useState } from "react";

import {
  ACCOUNT_PAGE,
  accountQuery,
  api,
  parseAccountFilter,
  SignedOut,
  type AccountFilter,
  type AccountPage,
  type AccountRow,
  type AccountSort,
} from "../api";
import { formatAgo, formatCount, formatWhen, t, type Lang } from "../i18n";
import { adminPath, navigate } from "../router";
import { Badge, describe, go, REFRESH_MS, useNow, useVisibleInterval } from "./shared";

const SORTS: AccountSort[] = ["newest", "oldest", "name", "games", "active"];

function pathFor(filter: AccountFilter): string {
  const query = accountQuery(filter);
  return `${adminPath("accounts")}${query ? `?${query}` : ""}`;
}

/** Where an account is now, as badges; nothing when it is offline. */
export function Presence({ lang, row }: { lang: Lang; row: Pick<AccountRow, "online" | "playing" | "in_lobby"> }) {
  if (!row.online) return null;
  if (row.playing) {
    return (
      <Badge tone="ok" dot>
        {t(lang, "acc.inGame")}
      </Badge>
    );
  }
  return (
    <Badge tone={row.in_lobby ? "info" : "warn"} dot>
      {t(lang, row.in_lobby ? "acc.online" : "live.atTable")}
    </Badge>
  );
}

function Row({ lang, row, now }: { lang: Lang; row: AccountRow; now: number }) {
  const to = adminPath("accounts", row.id);
  return (
    <tr data-testid="account">
      <td className="acc-name" data-label={t(lang, "acc.name")}>
        <a href={to} onClick={go(to)} className="acc-handle">
          {row.handle}
        </a>
        <span className="acc-badges">
          <Presence lang={lang} row={row} />
          {row.guest ? <Badge>{t(lang, "acc.guest")}</Badge> : null}
          {row.by_key && <Badge tone="accent">{t(lang, "acc.byKey")}</Badge>}
        </span>
        {row.username !== null && <span className="muted small mono">@{row.username}</span>}
      </td>
      <td data-label={t(lang, "acc.created")}>
        <span title={formatWhen(lang, row.created_at)}>{formatAgo(lang, row.created_at, now)}</span>
      </td>
      <td data-label={t(lang, "acc.active")}>
        {row.active_at === null ? (
          <span className="muted">{t(lang, "acc.never")}</span>
        ) : (
          <span title={formatWhen(lang, row.active_at)}>{formatAgo(lang, row.active_at, now)}</span>
        )}
      </td>
      <td className="num" data-label={t(lang, "acc.games")}>
        {formatCount(lang, row.games)}
      </td>
      <td className="num" data-label={t(lang, "acc.decks")}>
        {formatCount(lang, row.decks)}
      </td>
      <td className="num" data-label={t(lang, "acc.sessions")}>
        {formatCount(lang, row.sessions)}
      </td>
    </tr>
  );
}

/** Asks for the page `key` names, and hands over what came back. */
function fetchPage(
  key: string,
  lang: Lang,
  setPage: (page: { key: string; page: AccountPage }) => void,
  setError: (error: string | null) => void,
): void {
  api.admin
    .accounts(parseAccountFilter(key))
    .then((page) => {
      setPage({ key, page });
      setError(null);
    })
    .catch((e: unknown) => {
      if (!(e instanceof SignedOut)) setError(describe(lang, e));
    });
}

export function Accounts({ lang, search }: { lang: Lang; search: string }) {
  const filter = parseAccountFilter(search);
  const key = accountQuery(filter);
  const q = filter.q ?? "";
  const [page, setPage] = useState<{ key: string; page: AccountPage } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [text, setText] = useState(q);
  const [shownQ, setShownQ] = useState(q);
  const now = useNow();

  // The box follows the query string when it changes from elsewhere (back,
  // a link, the clear button).
  if (q !== shownQ) {
    setShownQ(q);
    setText(q);
  }

  // Asked every so often, and again whenever the query string changes.
  useVisibleInterval(() => {
    fetchPage(key, lang, setPage, setError);
  }, REFRESH_MS * 2);
  const fetched = useRef(key);
  useEffect(() => {
    if (fetched.current === key) return;
    fetched.current = key;
    fetchPage(key, lang, setPage, setError);
  }, [key, lang]);

  // The search box writes the query string a moment after typing stops.
  useEffect(() => {
    if (text === q) return;
    const timer = setTimeout(() => {
      const next: AccountFilter = { ...parseAccountFilter(key), q: text };
      delete next.offset;
      navigate(pathFor(next), true);
    }, 300);
    return () => {
      clearTimeout(timer);
    };
  }, [text, q, key]);

  const change = (patch: Partial<AccountFilter>) => {
    const next: AccountFilter = { ...filter, ...patch };
    if (!("offset" in patch)) delete next.offset;
    navigate(pathFor(next), true);
  };

  const busy = page !== null && page.key !== key;
  const current = page?.page ?? null;
  const offset = filter.offset ?? 0;
  const total = current?.total ?? 0;
  const rows = current?.accounts ?? [];
  const kind = filter.kind ?? "all";

  return (
    <>
      <div className="page-head">
        <div>
          <h1>{t(lang, "sec.accounts")}</h1>
          <p className="muted small" aria-live="polite">
            {current === null
              ? t(lang, "admin.loading")
              : total === 0
                ? t(lang, "acc.none")
                : t(lang, "acc.count", {
                    from: formatCount(lang, offset + 1),
                    to: formatCount(lang, offset + rows.length),
                    total: formatCount(lang, total),
                  })}
          </p>
        </div>
      </div>
      <form
        className="toolbar"
        aria-label={t(lang, "acc.searchLabel")}
        onSubmit={(event) => {
          event.preventDefault();
          change({ q: text });
        }}
      >
        <input
          type="search"
          className="grow"
          aria-label={t(lang, "acc.searchLabel")}
          placeholder={t(lang, "acc.search")}
          value={text}
          onChange={(event) => {
            setText(event.target.value);
          }}
        />
        <fieldset className="segmented">
<legend className="sr-only">{t(lang, "acc.kind")}</legend>
          {(["all", "registered", "guest"] as const).map((k) => (
            <button
              key={k}
              type="button"
              aria-pressed={kind === k}
              onClick={() => {
                change({ kind: k });
              }}
            >
              {t(lang, k === "all" ? "acc.kindAll" : k === "registered" ? "acc.kindRegistered" : "acc.kindGuest")}
            </button>
          ))}
        </fieldset>
        <button
          type="button"
          className="toggle"
          aria-pressed={filter.online === true}
          onClick={() => {
            change({ online: filter.online !== true });
          }}
        >
          <span className="presence-dot" aria-hidden="true" />
          {t(lang, "acc.onlineOnly")}
          {current !== null && <span className="seg-count">{formatCount(lang, current.online)}</span>}
        </button>
        <label className="select-label">
          <span className="sr-only">{t(lang, "acc.sort")}</span>
          <select
            aria-label={t(lang, "acc.sort")}
            value={filter.sort ?? "newest"}
            onChange={(event) => {
              change({ sort: event.target.value as AccountSort });
            }}
          >
            {SORTS.map((s) => (
              <option key={s} value={s}>
                {t(lang, `sort.${s}`)}
              </option>
            ))}
          </select>
        </label>
        {(key !== "" || text !== "") && (
          <button
            type="button"
            className="quiet"
            onClick={() => {
              setText("");
              navigate(adminPath("accounts"), true);
            }}
          >
            {t(lang, "acc.clear")}
          </button>
        )}
      </form>
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <div className={`table-wrap accounts-wrap${busy ? " busy" : ""}`} aria-busy={busy}>
        <table className="accounts">
          <thead>
            <tr>
              <th scope="col">{t(lang, "acc.name")}</th>
              <th scope="col">{t(lang, "acc.created")}</th>
              <th scope="col" title={t(lang, "acc.activeHint")}>
                {t(lang, "acc.active")}
              </th>
              <th scope="col" className="num">
                {t(lang, "acc.games")}
              </th>
              <th scope="col" className="num">
                {t(lang, "acc.decks")}
              </th>
              <th scope="col" className="num">
                {t(lang, "acc.sessions")}
              </th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <Row key={row.id} lang={lang} row={row} now={now} />
            ))}
          </tbody>
        </table>
        {current !== null && rows.length === 0 && <p className="empty">{t(lang, "acc.none")}</p>}
      </div>
      <p className="muted small hint">{t(lang, "acc.activeHint")}</p>
      {total > ACCOUNT_PAGE && (
        <nav className="pager" aria-label={t(lang, "sec.accounts")}>
          <button
            type="button"
            disabled={offset === 0}
            onClick={() => {
              change({ offset: Math.max(0, offset - ACCOUNT_PAGE) });
            }}
          >
            ← {t(lang, "acc.prev")}
          </button>
          <span className="muted small">
            {formatCount(lang, Math.floor(offset / ACCOUNT_PAGE) + 1)} /{" "}
            {formatCount(lang, Math.ceil(total / ACCOUNT_PAGE))}
          </span>
          <button
            type="button"
            disabled={offset + ACCOUNT_PAGE >= total}
            onClick={() => {
              change({ offset: offset + ACCOUNT_PAGE });
            }}
          >
            {t(lang, "acc.next")} →
          </button>
        </nav>
      )}
    </>
  );
}
