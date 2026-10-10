// One account (`/admin/accounts/{id}`): who it is, where it is now, its
// decks (never their cards) and its latest games with whom it played.

import { useCallback, useState } from "react";

import { api, Refused, SignedOut, type AccountDetail as Detail } from "../api";
import { formatAgo, formatCount, formatDuration, formatWhen, t, type Lang } from "../i18n";
import { adminPath } from "../router";
import { Presence } from "./Accounts";
import { Badge, copy, describe, Facts, Link, Panel, REFRESH_MS, useNow, useVisibleInterval } from "./shared";

function seconds(from: string, to: string): number {
  return (new Date(to).getTime() - new Date(from).getTime()) / 1000;
}

export function AccountDetail({ lang, id }: { lang: Lang; id: string }) {
  const [detail, setDetail] = useState<Detail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const now = useNow();

  const load = useCallback(() => {
    api.admin
      .account(id)
      .then((d) => {
        setDetail(d);
        setError(null);
      })
      .catch((e: unknown) => {
        if (e instanceof SignedOut) return;
        setError(e instanceof Refused && e.status === 404 ? t(lang, "det.notFound") : describe(lang, e));
      });
  }, [id, lang]);
  useVisibleInterval(load, REFRESH_MS * 2);

  const when = (iso: string) => (
    <span title={formatWhen(lang, iso)}>
      {formatWhen(lang, iso)} <span className="muted">({formatAgo(lang, iso, now)})</span>
    </span>
  );

  return (
    <>
      <p className="crumbs">
        <Link to={adminPath("accounts")}>← {t(lang, "det.back")}</Link>
      </p>
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {detail !== null && (
        <>
          <div className="page-head account-head">
            <div>
              <h1>{detail.handle}</h1>
              <p className="acc-badges">
                <Presence
                  lang={lang}
                  row={{ online: detail.online, playing: detail.table?.state === "playing", in_lobby: detail.in_lobby }}
                />
                {!detail.online && <Badge>{t(lang, "det.offline")}</Badge>}
                <Badge tone={detail.guest ? "muted" : "accent"}>
                  {t(lang, detail.guest ? "acc.guest" : "acc.registered")}
                </Badge>
                {detail.by_key && <Badge tone="accent">{t(lang, "acc.byKey")}</Badge>}
              </p>
            </div>
            <button
              type="button"
              onClick={() => {
                void copy(detail.id).then(setCopied);
              }}
            >
              {copied ? t(lang, "det.copied") : t(lang, "det.copy")}
            </button>
          </div>
          <div className="kpis">
            <div className="kpi">
              <span className="kpi-label">{t(lang, "acc.games")}</span>
              <span className="kpi-value">{formatCount(lang, detail.games)}</span>
            </div>
            <div className="kpi">
              <span className="kpi-label">{t(lang, "acc.decks")}</span>
              <span className="kpi-value">{formatCount(lang, detail.decks)}</span>
            </div>
            <div className="kpi">
              <span className="kpi-label">{t(lang, "acc.sessions")}</span>
              <span className="kpi-value">{formatCount(lang, detail.sessions)}</span>
            </div>
            <div className="kpi">
              <span className="kpi-label">{t(lang, "acc.active")}</span>
              <span className="kpi-value kpi-text">
                {detail.active_at === null ? t(lang, "acc.never") : formatAgo(lang, detail.active_at, now)}
              </span>
            </div>
          </div>
          {detail.table !== null && (
            <p className="notice now-at">
              {t(lang, "det.now")}: <strong>{detail.table.name || `#${detail.table.id.slice(-8)}`}</strong>{" "}
              <Badge tone={detail.table.state === "playing" ? "ok" : "warn"} dot>
                {t(lang, detail.table.state === "playing" ? "live.playing" : "live.waiting")}
              </Badge>{" "}
              <Link to={`${adminPath("live")}#table-${detail.table.id}`}>{t(lang, "det.openTable")}</Link>
            </p>
          )}
          <div className="panels detail-panels">
            <Panel id="det-facts" title={t(lang, "acc.name")}>
              <Facts
                rows={[
                  [t(lang, "det.handle"), detail.handle],
                  [t(lang, "det.username"), detail.username ?? "—"],
                  [t(lang, "det.id"), <span key="id" className="mono small break">{detail.id}</span>],
                  [t(lang, "acc.created"), when(detail.created_at)],
                  [t(lang, "acc.active"), detail.active_at === null ? "—" : when(detail.active_at)],
                  [t(lang, "det.lang"), detail.lang || "—"],
                  [
                    t(lang, "det.email"),
                    !detail.has_email
                      ? t(lang, "det.emailNone")
                      : detail.confirmed_at !== null
                        ? t(lang, "det.emailConfirmed", { when: formatWhen(lang, detail.confirmed_at) })
                        : t(lang, "det.emailUnconfirmed"),
                  ],
                  [
                    t(lang, "det.terms"),
                    detail.terms_version === null
                      ? t(lang, "det.termsNone")
                      : t(lang, "det.termsAccepted", {
                          version: detail.terms_version,
                          when: detail.terms_accepted_at === null ? "" : formatWhen(lang, detail.terms_accepted_at),
                        }),
                  ],
                  [
                    t(lang, "det.key"),
                    detail.by_key ? (detail.key_note ?? t(lang, "det.keyUsed")) : t(lang, "det.keyNone"),
                  ],
                ]}
              />
              <p className="muted small">{t(lang, "acc.activeHint")}</p>
            </Panel>
            <Panel id="det-decks" title={`${t(lang, "det.decks")} · ${formatCount(lang, detail.deck_list.length)}`}>
              {detail.deck_list.length === 0 ? (
                <p className="empty">{t(lang, "det.noDecks")}</p>
              ) : (
                <ul className="rows">
                  {detail.deck_list.map((deck) => (
                    <li key={deck.id} className="row-item">
                      <strong>{deck.name}</strong>
                      <span className="muted small">
                        {deck.format} · {t(lang, "det.cards", { n: formatCount(lang, deck.cards) })} ·{" "}
                        {t(lang, "det.version", { n: deck.version })} ·{" "}
                        {t(lang, "det.updated", { when: formatAgo(lang, deck.updated_at, now) })}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </Panel>
            <Panel id="det-games" title={t(lang, "det.games")} className="wide-panel">
              {detail.recent_games.length === 0 ? (
                <p className="empty">{t(lang, "det.noGames")}</p>
              ) : (
                <ul className="rows">
                  {detail.recent_games.map((game) => (
                    <li key={game.game_id} className="row-item game-item">
                      <span className="game-when">
                        <strong title={formatWhen(lang, game.started_at)}>{formatAgo(lang, game.started_at, now)}</strong>
                        <span className="muted small">
                          {game.ended_at === null
                            ? t(lang, "det.incomplete")
                            : `${t(lang, game.complete ? "det.complete" : "det.incomplete")} · ${t(lang, "det.lasted", {
                                d: formatDuration(lang, seconds(game.started_at, game.ended_at)),
                              })}`}
                        </span>
                      </span>
                      <span className="game-chairs">
                        {game.chairs.map((chair) => (
                          <span
                            key={chair.seat}
                            className={chair.seat === game.seat ? "chair-pill self" : "chair-pill"}
                          >
                            {chair.player ?? t(lang, "det.house")}
                          </span>
                        ))}
                      </span>
                      <span className="mono small muted game-id">{game.game_id.slice(-8)}</span>
                    </li>
                  ))}
                </ul>
              )}
            </Panel>
          </div>
        </>
      )}
    </>
  );
}
