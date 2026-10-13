// The blocked cards (`/admin/cards`): a card of the gateway's pool found
// by name and blocked, without an end or until a moment, with a reason the
// players see; the blocks in force, each lifted after a confirmation; and
// the earlier ones. Blocking and lifting both ask first.

import { useCallback, useEffect, useState, type FormEvent } from "react";

import { api, SignedOut, type BlockOrder, type CardBlock, type CardHit, type ConsoleChange } from "../api";
import { formatAgo, formatCount, formatWhen, t, type Key, type Lang } from "../i18n";
import { CardChip } from "../reports/pieces";
import { Badge, describe, Panel, useNow } from "./shared";

/** How long the form offers, in hours; `0` for no end, `-1` for a moment picked. */
const DURATIONS: { hours: number; label: Key }[] = [
  { hours: 0, label: "cards.forever" },
  { hours: 24, label: "cards.day1" },
  { hours: 24 * 7, label: "cards.days7" },
  { hours: 24 * 30, label: "cards.days30" },
  { hours: -1, label: "cards.untilDate" },
];

/** How long a typed search waits for the next key before it asks. */
const SEARCH_DELAY_MS = 250;

const pad = (n: number) => String(n).padStart(2, "0");

/** `datetime-local`'s value for `at`, in the browser's time zone. */
function localInput(at: Date): string {
  return `${at.getFullYear()}-${pad(at.getMonth() + 1)}-${pad(at.getDate())}T${pad(at.getHours())}:${pad(at.getMinutes())}`;
}

/** The form under a found card: reason, how long, then a confirmation. */
function BlockForm({
  lang,
  card,
  onBlocked,
  onCancel,
}: {
  lang: Lang;
  card: CardHit;
  onBlocked: (block: CardBlock) => void;
  onCancel: () => void;
}) {
  const [reason, setReason] = useState("");
  const [hours, setHours] = useState(0);
  const [moment, setMoment] = useState(() => localInput(new Date(Date.now() + 24 * 3600 * 1000)));
  const [confirm, setConfirm] = useState<BlockOrder | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const order = (): BlockOrder | null => {
    const base: BlockOrder = { index: card.index, reason: reason.trim() };
    if (hours === 0) return base;
    const until = hours > 0 ? new Date(Date.now() + hours * 3600 * 1000) : new Date(moment);
    if (Number.isNaN(until.getTime()) || until.getTime() <= Date.now()) {
      setError(t(lang, "cards.untilPast"));
      return null;
    }
    return { ...base, until: until.toISOString() };
  };

  const ask = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    setConfirm(order());
  };

  const block = async (wanted: BlockOrder) => {
    setBusy(true);
    setError(null);
    try {
      onBlocked(await api.admin.cards.block(wanted));
    } catch (e: unknown) {
      if (!(e instanceof SignedOut)) setError(describe(lang, e));
      setConfirm(null);
    } finally {
      setBusy(false);
    }
  };

  const id = `block-${card.index}`;
  return (
    <form className="block-form" onSubmit={ask} aria-label={`${t(lang, "cards.block")} ${card.name}`}>
      <div className="form-grid">
        <div className="field wide">
          <label htmlFor={`${id}-reason`}>{t(lang, "cards.reason")}</label>
          <input
            id={`${id}-reason`}
            required
            maxLength={200}
            autoComplete="off"
            placeholder={t(lang, "cards.reasonPlaceholder")}
            value={reason}
            disabled={confirm !== null}
            onChange={(event) => {
              setReason(event.target.value);
            }}
          />
        </div>
        <div className="field">
          <label htmlFor={`${id}-hours`}>{t(lang, "cards.duration")}</label>
          <select
            id={`${id}-hours`}
            value={hours}
            disabled={confirm !== null}
            onChange={(event) => {
              setHours(Number(event.target.value));
            }}
          >
            {DURATIONS.map((d) => (
              <option key={d.hours} value={d.hours}>
                {t(lang, d.label)}
              </option>
            ))}
          </select>
        </div>
        {hours === -1 && (
          <div className="field">
            <label htmlFor={`${id}-until`}>{t(lang, "cards.until")}</label>
            <input
              id={`${id}-until`}
              type="datetime-local"
              required
              value={moment}
              disabled={confirm !== null}
              onChange={(event) => {
                setMoment(event.target.value);
              }}
            />
          </div>
        )}
      </div>
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {confirm === null ? (
        <div className="row-actions">
          <button type="submit" className="danger" disabled={reason.trim() === ""}>
            {t(lang, "cards.block")}
          </button>
          <button type="button" onClick={onCancel}>
            {t(lang, "cards.blockNo")}
          </button>
        </div>
      ) : (
        <fieldset className="confirm">
          <legend>
            {t(lang, "cards.blockConfirm", {
              name: card.name,
              when:
                confirm.until === undefined
                  ? t(lang, "cards.whenForever")
                  : t(lang, "cards.whenUntil", {
                      at: formatWhen(lang, confirm.until),
                    }),
            })}
          </legend>
          <div className="row-actions">
            <button type="button" className="danger" disabled={busy} onClick={() => void block(confirm)}>
              {busy ? t(lang, "cards.blocking") : t(lang, "cards.blockYes")}
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => {
                setConfirm(null);
              }}
            >
              {t(lang, "cards.blockNo")}
            </button>
          </div>
        </fieldset>
      )}
    </form>
  );
}

/** What a found card's block is: the list's word when it has been read, which
 * a block made or lifted here changes at once, else the search's. */
function blockOf(card: CardHit, active: CardBlock[] | null): { reason: string; until: string | null } | null {
  if (active === null) return card.blocked;
  return active.find((b) => b.index === card.index) ?? null;
}

/** The search and what it found, each card with its block or a way to make one. */
function Search({
  lang,
  active,
  onBlocked,
}: {
  lang: Lang;
  active: CardBlock[] | null;
  onBlocked: (block: CardBlock) => void;
}) {
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<CardHit[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState<number | null>(null);

  useEffect(() => {
    const q = query.trim();
    if (q === "") return;
    let current = true;
    const timer = setTimeout(() => {
      setBusy(true);
      api.admin.cards
        .search(q)
        .then((found) => {
          if (!current) return;
          setHits(found);
          setError(null);
        })
        .catch((e: unknown) => {
          if (current && !(e instanceof SignedOut)) setError(describe(lang, e));
        })
        .finally(() => {
          if (current) setBusy(false);
        });
    }, SEARCH_DELAY_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [query, lang]);
  const found = query.trim() === "" ? null : hits;

  return (
    <section className="card card-search" aria-labelledby="card-search-title">
      <h3 id="card-search-title">{t(lang, "cards.search")}</h3>
      <div className="field">
        <label htmlFor="card-search">{t(lang, "cards.searchLabel")}</label>
        <input
          id="card-search"
          type="search"
          autoComplete="off"
          placeholder={t(lang, "cards.searchPlaceholder")}
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setOpen(null);
          }}
        />
      </div>
      <p className="muted small">{t(lang, "cards.hint")}</p>
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {busy && found === null && <p className="muted">{t(lang, "cards.searching")}</p>}
      {found !== null &&
        (found.length === 0 ? (
          <p className="empty">{t(lang, "cards.noHits")}</p>
        ) : (
          <ul className="card-hits">
            {found.map((card) => {
              const blocked = blockOf(card, active);
              return (
                <li key={card.index} className="card-hit" data-testid="card-hit">
                  <div className="card-hit-head">
                    <CardChip name={card.name} scryfallId={card.scryfall_id || null} lang={lang} />
                    <span className="muted small">{card.type_line}</span>
                    <span className="muted small mono">#{card.index}</span>
                    {blocked !== null ? (
                      <Badge tone="danger">
                        {t(lang, "cards.blockedNow")}
                        {blocked.until !== null && ` · ${formatWhen(lang, blocked.until)}`}
                      </Badge>
                    ) : (
                      open !== card.index && (
                        <button
                          type="button"
                          className="danger-quiet"
                          aria-label={`${t(lang, "cards.block")} ${card.name}`}
                          onClick={() => {
                            setOpen(card.index);
                          }}
                        >
                          {t(lang, "cards.block")}
                        </button>
                      )
                    )}
                  </div>
                  {blocked !== null && <p className="muted small">{blocked.reason}</p>}
                  {open === card.index && blocked === null && (
                    <BlockForm
                      lang={lang}
                      card={card}
                      onCancel={() => {
                        setOpen(null);
                      }}
                      onBlocked={(block) => {
                        setOpen(null);
                        onBlocked(block);
                      }}
                    />
                  )}
                </li>
              );
            })}
          </ul>
        ))}
    </section>
  );
}

/** One block: what, why, since when and until when; Unblock while in force. */
function BlockItem({
  lang,
  block,
  onLifted,
  onError,
}: {
  lang: Lang;
  block: CardBlock;
  onLifted: () => void;
  onError: (message: string) => void;
}) {
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const lift = async () => {
    setBusy(true);
    try {
      await api.admin.cards.unblock(block.id);
      setConfirm(false);
      onLifted();
    } catch (e: unknown) {
      if (!(e instanceof SignedOut)) onError(describe(lang, e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <li className={`invite block state-${block.state}`} data-testid="card-block">
      <div className="invite-head">
        <span className="invite-note">
          <CardChip name={block.name} scryfallId={null} lang={lang} />
        </span>
        <span className={`tag invite-state state-${block.state === "active" ? "active" : "revoked"}`}>
          {t(lang, `cardstate.${block.state}`)}
        </span>
      </div>
      <p className="block-reason">{block.reason}</p>
      <dl className="facts invite-facts">
        <div className="fact">
          <dt>{t(lang, "cards.ends")}</dt>
          <dd>{block.until === null ? t(lang, "cards.never") : formatWhen(lang, block.until)}</dd>
        </div>
        <div className="fact">
          <dt>{t(lang, "cards.made")}</dt>
          <dd>{formatWhen(lang, block.created_at)}</dd>
        </div>
        <div className="fact">
          <dt>{t(lang, "cards.by")}</dt>
          <dd>{block.created_by}</dd>
        </div>
        {block.lifted_at !== null && (
          <div className="fact">
            <dt>{t(lang, "cards.lifted")}</dt>
            <dd>
              {formatWhen(lang, block.lifted_at)}
              {block.lifted_by !== null && ` · ${block.lifted_by}`}
            </dd>
          </div>
        )}
      </dl>
      {block.state === "active" &&
        (confirm ? (
          <fieldset className="confirm">
            <legend>{t(lang, "cards.unblockConfirm", { name: block.name })}</legend>
            <div className="row-actions">
              <button type="button" className="danger" disabled={busy} onClick={() => void lift()}>
                {t(lang, "cards.unblockYes")}
              </button>
              <button
                type="button"
                disabled={busy}
                onClick={() => {
                  setConfirm(false);
                }}
              >
                {t(lang, "cards.unblockNo")}
              </button>
            </div>
          </fieldset>
        ) : (
          <button
            type="button"
            className="danger-quiet"
            aria-label={`${t(lang, "cards.unblock")} ${block.name}`}
            onClick={() => {
              setConfirm(true);
            }}
          >
            {t(lang, "cards.unblock")}
          </button>
        ))}
    </li>
  );
}

export function Cards({ lang }: { lang: Lang }) {
  const [blocks, setBlocks] = useState<CardBlock[] | null>(null);
  const [changes, setChanges] = useState<ConsoleChange[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const now = useNow();

  const load = useCallback(() => {
    api.admin.cards
      .blocks()
      .then((list) => {
        setBlocks(list);
        setError(null);
      })
      .catch((e: unknown) => {
        if (!(e instanceof SignedOut)) setError(describe(lang, e));
      });
    api.admin
      .audit()
      .then((all) => {
        setChanges(all.filter((c) => c.action.startsWith("gateway.card.")));
      })
      .catch(() => {
        // The changes are a footnote; the page reads without them.
      });
  }, [lang]);
  useEffect(load, [load]);

  const changed = (said: string) => {
    setNotice(said);
    load();
  };

  const active = (blocks ?? []).filter((b) => b.state === "active");
  const earlier = (blocks ?? []).filter((b) => b.state !== "active");
  const list = (shown: CardBlock[], empty: Key) =>
    shown.length === 0 ? (
      <p className="empty">{t(lang, empty)}</p>
    ) : (
      <ul className="invite-list">
        {shown.map((block) => (
          <BlockItem
            key={block.id}
            lang={lang}
            block={block}
            onLifted={() => {
              changed(t(lang, "cards.unblocked"));
            }}
            onError={setError}
          />
        ))}
      </ul>
    );

  return (
    <>
      <div className="page-head">
        <div>
          <h1>{t(lang, "cards.title")}</h1>
          {blocks !== null && (
            <p className="muted small">
              {t(lang, "cards.summary", {
                n: formatCount(lang, active.length),
              })}
            </p>
          )}
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
      <section className="invites card-blocks" aria-labelledby="blocks-title">
        <h2 id="blocks-title" className="sr-only">
          {t(lang, "cards.title")}
        </h2>
        <Search
          lang={lang}
          active={blocks === null ? null : active}
          onBlocked={(block) => {
            changed(t(lang, "cards.blockedDone", { name: block.name }));
          }}
        />
        {notice !== null && <output className="notice">{notice}</output>}
        <div className="panel-head list-head">
          <h3>{t(lang, "cards.active")}</h3>
        </div>
        {blocks === null ? <p className="muted">{t(lang, "admin.loading")}</p> : list(active, "cards.noneActive")}
        {blocks !== null && (
          <details className="block-history">
            <summary>
              {t(lang, "cards.history")} <span className="seg-count">{formatCount(lang, earlier.length)}</span>
            </summary>
            {list(earlier, "cards.noHistory")}
          </details>
        )}
        <Panel id="card-audit-title" title={t(lang, "audit.title")} className="audit-panel">
          {changes.length === 0 ? (
            <p className="empty">{t(lang, "audit.none")}</p>
          ) : (
            <ol className="audit console-audit">
              {changes.map((c) => (
                <li key={`${c.at}-${c.action}-${c.detail ?? ""}`}>
                  <span className="muted" title={formatWhen(lang, c.at)}>
                    {formatAgo(lang, c.at, now)}
                  </span>{" "}
                  <strong>{c.actor}</strong> <Badge>{c.action}</Badge>
                  {c.detail !== null && <span className="muted small audit-detail"> {c.detail}</span>}
                </li>
              ))}
            </ol>
          )}
        </Panel>
      </section>
    </>
  );
}
