// Live (`/admin/live`): who is online and where, every open and running
// table with its chairs, and the agents that run the games, asked again
// every five seconds while the page is visible.

import { useCallback, useState } from "react";

import { api, SignedOut, type Live as LiveData, type LiveTable, type OnlinePlayer } from "../api";
import { formatAgo, formatCount, t, type Lang } from "../i18n";
import { adminPath } from "../router";
import { Badge, describe, Freshness, Link, LIVE_MS, Panel, useNow, useVisibleInterval } from "./shared";

type TableFilter = "all" | "waiting" | "playing";

/** A player's name, linked to their account when the console knows it. */
function Who({ id, name, guest, lang }: { id: string | null; name: string | null; guest: boolean | null; lang: Lang }) {
  const label = name ?? "?";
  return (
    <span className="who-name">
      {id === null ? label : <Link to={adminPath("accounts", id)}>{label}</Link>}
      {guest === true && <Badge>{t(lang, "live.guest")}</Badge>}
    </span>
  );
}

/** What to call a table: its name, else who sits at it. */
export function tableName(lang: Lang, table: LiveTable): string {
  if (table.name !== "") return table.name;
  const sitters = table.seats.map((s) =>
    s.kind === "ai" ? t(lang, "det.houseShort") : (s.player ?? s.bridge ?? t(lang, "live.open")),
  );
  return sitters.join(" vs. ");
}

export function TableCard({ lang, table, compact = false }: { lang: Lang; table: LiveTable; compact?: boolean }) {
  const now = useNow();
  const taken = table.seats.filter((s) => s.kind === "ai" || s.account_id !== null || s.bridge !== null).length;
  return (
    <article className={`table-card state-${table.state}`} data-testid="table" id={`table-${table.id}`}>
      <header className="table-card-head">
        <h3>{tableName(lang, table)}</h3>
        <Badge tone={table.state === "playing" ? "ok" : "warn"} dot>
          {t(lang, table.state === "playing" ? "live.playing" : "live.waiting")}
        </Badge>
        {table.locked && <Badge>🔒 {t(lang, "live.locked")}</Badge>}
        {table.rematch && <Badge tone="info">{t(lang, "live.rematch")}</Badge>}
      </header>
      <p className="muted small table-meta">
        {t(lang, "live.opened", { when: formatAgo(lang, table.created_at, now) })} ·{" "}
        {t(lang, "live.seats", { taken: formatCount(lang, taken), total: formatCount(lang, table.seats.length) })}
        {!compact && (
          <>
            {" · "}
            {table.decide_secs > 0
              ? t(lang, "live.clock", { secs: formatCount(lang, table.decide_secs) })
              : t(lang, "live.noClock")}
          </>
        )}
      </p>
      <ol className="chairs">
        {table.seats.map((seat) => (
          <li key={seat.seat} className="chair">
            <span className="chair-no" aria-hidden="true">
              {seat.seat + 1}
            </span>
            <span className="chair-body">
              {seat.kind === "ai" ? (
                <span className="who-name">🤖 {t(lang, "live.ai", { level: seat.ai ?? "?" })}</span>
              ) : seat.bridge !== null ? (
                <span className="who-name">
                  {t(lang, "live.bridge", { name: seat.bridge, by: seat.bridged_by ?? "?" })}
                </span>
              ) : seat.account_id === null ? (
                <span className="muted">{t(lang, "live.open")}</span>
              ) : (
                <Who lang={lang} id={seat.account_id} name={seat.player} guest={seat.guest} />
              )}
              {!compact && seat.deck !== "" && (
                <span className="muted small chair-deck">
                  {seat.deck}
                  {seat.format !== null ? ` · ${seat.format}` : ""}
                </span>
              )}
            </span>
            {table.state === "waiting" && seat.kind === "human" && seat.account_id !== null && (
              <Badge tone={seat.ready ? "ok" : "muted"}>{t(lang, seat.ready ? "live.ready" : "live.notReady")}</Badge>
            )}
            {seat.account_id !== null && seat.account_id === table.host_id && <Badge tone="accent">{t(lang, "live.host")}</Badge>}
            {seat.team !== null && <Badge tone="info">{t(lang, "live.team", { n: seat.team })}</Badge>}
          </li>
        ))}
      </ol>
      {!compact && table.state === "playing" && (
        <p className="muted small table-meta">
          {table.engine ? t(lang, "live.engine") : t(lang, "live.noEngine")}
          {table.agent !== null && ` · ${table.agent}`}
          {table.engine_local && ` · ${t(lang, "live.local")}`}
        </p>
      )}
    </article>
  );
}

function PlayerItem({ lang, player }: { lang: Lang; player: OnlinePlayer }) {
  return (
    <li className="player" data-testid="online-player">
      <span className="presence-dot" aria-hidden="true" />
      <Who lang={lang} id={player.id} name={player.handle} guest={player.guest} />
      <span className="player-where">
        {player.playing !== null ? (
          <a href={`#table-${player.playing}`}>
            <Badge tone="ok">{t(lang, "live.inGame")}</Badge>
          </a>
        ) : player.waiting !== null ? (
          <a href={`#table-${player.waiting}`}>
            <Badge tone="warn">{t(lang, "live.atTable")}</Badge>
          </a>
        ) : null}
        {player.in_lobby && <Badge tone="info">{t(lang, "live.inLobby")}</Badge>}
      </span>
    </li>
  );
}

export function Live({ lang }: { lang: Lang }) {
  const [live, setLive] = useState<LiveData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<TableFilter>("all");
  const [search, setSearch] = useState("");

  const load = useCallback(() => {
    api.admin
      .live()
      .then((l) => {
        setLive(l);
        setError(null);
      })
      .catch((e: unknown) => {
        if (!(e instanceof SignedOut)) setError(describe(lang, e));
      });
  }, [lang]);
  const shown = useVisibleInterval(load, LIVE_MS);

  const tables = (live?.tables ?? []).filter((table) => filter === "all" || table.state === filter);
  const needle = search.trim().toLowerCase();
  const players = (live?.players ?? [])
    .filter((p) => needle === "" || (p.handle ?? "").toLowerCase().includes(needle))
    .sort((a, b) => (a.handle ?? "").localeCompare(b.handle ?? ""));
  const count = (state: TableFilter) =>
    (live?.tables ?? []).filter((table) => state === "all" || table.state === state).length;

  return (
    <>
      <div className="page-head">
        <div>
          <h1>{t(lang, "sec.live")}</h1>
          <Freshness lang={lang} at={live?.at ?? null} shown={shown} />
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
      {live !== null && (
        <div className="live-layout">
          <Panel
            id="live-players"
            title={`${t(lang, "live.players")} · ${formatCount(lang, live.players.length)}`}
            className="live-players"
          >
            {live.players.length > 6 && (
              <input
                type="search"
                className="full"
                aria-label={t(lang, "live.search")}
                placeholder={t(lang, "live.search")}
                value={search}
                onChange={(event) => {
                  setSearch(event.target.value);
                }}
              />
            )}
            {live.players.length === 0 ? (
              <p className="empty">{t(lang, "live.noPlayers")}</p>
            ) : players.length === 0 ? (
              <p className="empty">{t(lang, "live.noMatch")}</p>
            ) : (
              <ul className="players">
                {players.map((p) => (
                  <PlayerItem key={p.id} lang={lang} player={p} />
                ))}
              </ul>
            )}
          </Panel>
          <Panel
            id="live-tables"
            title={t(lang, "live.tables")}
            className="live-tables"
            actions={
              <fieldset className="segmented">
<legend className="sr-only">{t(lang, "live.tables")}</legend>
                {(["all", "waiting", "playing"] as const).map((f) => (
                  <button
                    key={f}
                    type="button"
                    aria-pressed={filter === f}
                    onClick={() => {
                      setFilter(f);
                    }}
                  >
                    {f === "all" ? t(lang, "live.all") : t(lang, f === "waiting" ? "live.waiting" : "live.playing")}{" "}
                    <span className="seg-count">{formatCount(lang, count(f))}</span>
                  </button>
                ))}
              </fieldset>
            }
          >
            {tables.length === 0 ? (
              <p className="empty">{t(lang, "live.noTables")}</p>
            ) : (
              <div className="table-cards">
                {tables.map((table) => (
                  <TableCard key={table.id} lang={lang} table={table} />
                ))}
              </div>
            )}
          </Panel>
          <Panel id="live-agents" title={t(lang, "live.agents")} className="live-agents">
            {live.agents.length === 0 ? (
              <p className="empty warn-text">{t(lang, "live.noAgents")}</p>
            ) : (
              <ul className="agents">
                {live.agents.map((agent) => (
                  <li key={agent.id}>
                    <span className="presence-dot" aria-hidden="true" />
                    <strong>{agent.name}</strong>
                    {agent.local && <Badge tone="info">{t(lang, "live.local")}</Badge>}
                    <span className="muted small">
                      {t(lang, "live.agentGames", { n: formatCount(lang, agent.games) })} ·{" "}
                      {agent.capacity === 0
                        ? t(lang, "live.unlimited")
                        : t(lang, "live.capacity", { n: formatCount(lang, agent.capacity) })}
                    </span>
                    {agent.capacity > 0 && (
                      <meter
                        min={0}
                        max={agent.capacity}
                        value={agent.games}
                        aria-label={t(lang, "live.capacity", { n: agent.capacity })}
                      />
                    )}
                  </li>
                ))}
              </ul>
            )}
          </Panel>
        </div>
      )}
    </>
  );
}
