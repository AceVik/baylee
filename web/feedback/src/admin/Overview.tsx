// The overview (`/admin`): the gateway's headline numbers, a month of days
// as charts (the reports' bugs and crashes among them), who is at which
// table right now, every count the gateway keeps, and whether the gateway
// and this service answer, refreshed every 15 seconds while the page is
// visible.

import { useCallback, useState } from "react";

import {
  api,
  SignedOut,
  type Facets,
  type Health,
  type Live,
  type ReportStats,
  type Since,
  type Stats,
} from "../api";
import {
  formatBytes,
  formatCount,
  formatDuration,
  t,
  type Lang,
} from "../i18n";
import { adminPath } from "../router";
import { DayChart, type DayOf } from "./Chart";
import {
  Badge,
  describe,
  Facts,
  Freshness,
  Kpi,
  Link,
  Panel,
  REFRESH_MS,
  useVisibleInterval,
} from "./shared";
import { TableCard } from "./Live";

/** The kinds the error-rate chart counts as a defect. */
const DEFECTS = new Set(["bug", "crash"]);

/** `YYYY-MM-DD` of a moment, UTC. */
function utcDay(at: Date): string {
  return at.toISOString().slice(0, 10);
}

/**
 * The report counts as one row per UTC day over the window, defects apart
 * from the rest, with every day present so a quiet day is a gap and not a
 * missing bar.
 */
export function reportDays(
  stats: ReportStats,
  today: Date = new Date(),
): DayOf<"bugs" | "other">[] {
  const days: DayOf<"bugs" | "other">[] = [];
  const byDay = new Map<string, DayOf<"bugs" | "other">>();
  for (let back = stats.days - 1; back >= 0; back -= 1) {
    const at = new Date(
      Date.UTC(
        today.getUTCFullYear(),
        today.getUTCMonth(),
        today.getUTCDate() - back,
      ),
    );
    const row = { day: utcDay(at), bugs: 0, other: 0 };
    days.push(row);
    byDay.set(row.day, row);
  }
  for (const r of stats.rows) {
    const row = byDay.get(r.day);
    if (row === undefined) continue;
    if (DEFECTS.has(r.kind)) row.bugs += r.count;
    else row.other += r.count;
  }
  return days;
}

/** The service itself and the gateway: versions, and whether each answers. */
function Services({
  lang,
  stats,
  health,
  latency,
}: {
  lang: Lang;
  stats: Stats;
  health: Health | null;
  latency: number | null;
}) {
  const reach = (ms: number | null) =>
    ms === null ? (
      <Badge key="gw-reach" tone="warn">
        {t(lang, "srv.unreachable")}
      </Badge>
    ) : (
      <Badge key="gw-reach" tone="ok" dot>
        {t(lang, "srv.reachable", { ms: formatCount(lang, Math.round(ms)) })}
      </Badge>
    );
  return (
    <Panel id="st-services" title={t(lang, "srv.services")}>
      <Facts
        rows={[
          [t(lang, "srv.gateway"), reach(latency)],
          ...(stats.gateway.uptime_secs === undefined
            ? []
            : [
                [
                  t(lang, "stats.uptime"),
                  formatDuration(lang, stats.gateway.uptime_secs),
                ] as [string, string],
              ]),
          [
            t(lang, "srv.feedback"),
            health === null ? (
              <Badge key="fb" tone="muted">
                {t(lang, "srv.checking")}
              </Badge>
            ) : health.ok ? (
              <Badge key="fb" tone="ok" dot>
                {t(lang, "stats.on")}
              </Badge>
            ) : (
              <Badge key="fb" tone="warn">
                {t(lang, "srv.unreachable")}
              </Badge>
            ),
          ],
        ]}
      />
      <h3>
        {t(lang, "srv.gateway")} · {t(lang, "stats.version")}
      </h3>
      <p className="mono small build">{stats.gateway.version}</p>
      {health?.ok === true && (
        <>
          <h3>
            {t(lang, "srv.feedback")} · {t(lang, "stats.version")}
          </h3>
          <p className="mono small build" data-testid="feedback-version">
            {health.version}
          </p>
        </>
      )}
    </Panel>
  );
}

function sinceRows(lang: Lang, since: Since): [string, string][] {
  return [
    [t(lang, "stats.today"), formatCount(lang, since.today_utc)],
    [t(lang, "stats.week"), formatCount(lang, since.last_7d)],
    [t(lang, "stats.month"), formatCount(lang, since.last_30d)],
  ];
}

function count(
  facets: Facets | null,
  list: "statuses" | "kinds",
  value: string,
): number {
  return facets?.[list].find((c) => c.value === value)?.count ?? 0;
}

function Headline({
  lang,
  stats,
  facets,
  live,
}: {
  lang: Lang;
  stats: Stats;
  facets: Facets | null;
  live: Live | null;
}) {
  const n = (v: number) => formatCount(lang, v);
  // The live list counts a chair at a waiting table too, as the Live page
  // does; the stats count lobby sockets and running games only.
  const online = live?.players.length ?? stats.online.players;
  const inLobby =
    live?.players.filter((p) => p.in_lobby).length ?? stats.online.in_lobby;
  const atTable =
    live?.players.filter((p) => p.playing !== null || p.waiting !== null)
      .length ?? stats.online.seated;
  const reports = facets?.statuses.reduce((sum, c) => sum + c.count, 0) ?? null;
  return (
    <div className="kpis">
      <Kpi
        live
        label={t(lang, "kpi.online")}
        value={n(online)}
        testId="online"
        to={adminPath("live")}
        sub={t(lang, "kpi.onlineSub", { lobby: n(inLobby), table: n(atTable) })}
      />
      <Kpi
        label={t(lang, "kpi.games")}
        value={n(stats.games.running)}
        testId="running"
        to={adminPath("live")}
        sub={t(lang, "kpi.gamesSub", { waiting: n(stats.games.waiting) })}
      />
      <Kpi
        label={t(lang, "kpi.accounts")}
        value={n(stats.accounts.registered + stats.guests.live)}
        testId="accounts"
        to={adminPath("accounts")}
        sub={t(lang, "kpi.accountsSub", {
          registered: n(stats.accounts.registered),
          guests: n(stats.guests.live),
        })}
      />
      <Kpi
        label={t(lang, "kpi.today")}
        value={n(
          stats.accounts.created.today_utc +
            (stats.guests.created?.today_utc ?? 0),
        )}
        sub={t(lang, "kpi.todaySub", {
          registered: n(stats.accounts.created.today_utc),
          guests: n(stats.guests.created?.today_utc ?? 0),
        })}
      />
      <Kpi
        label={t(lang, "kpi.played")}
        value={n(stats.games.started.today_utc)}
        sub={t(lang, "kpi.playedSub", {
          finished: n(stats.games.finished_since.today_utc),
        })}
      />
      {reports !== null && (
        <Kpi
          label={t(lang, "kpi.reports")}
          value={n(count(facets, "statuses", "new"))}
          to="/?status=new"
          sub={t(lang, "kpi.reportsSub", { total: n(reports) })}
        />
      )}
    </div>
  );
}

function Charts({
  lang,
  stats,
  reports,
}: {
  lang: Lang;
  stats: Stats;
  reports: ReportStats | null;
}) {
  const days = stats.daily ?? [];
  if (days.length === 0) return null;
  return (
    <div className="charts">
      {reports !== null && (
        <DayChart
          lang={lang}
          title={t(lang, "chart.reports")}
          days={reportDays(reports)}
          series={[
            { key: "bugs", label: t(lang, "chart.bugs"), hue: "c2" },
            { key: "other", label: t(lang, "chart.otherReports"), hue: "c1" },
          ]}
        />
      )}
      <DayChart
        lang={lang}
        title={t(lang, "chart.games")}
        days={days}
        series={[
          { key: "started", label: t(lang, "chart.started"), hue: "c1" },
        ]}
      />
      <DayChart
        lang={lang}
        title={t(lang, "chart.players")}
        days={days}
        series={[
          { key: "players", label: t(lang, "chart.playersSeries"), hue: "c1" },
        ]}
      />
      <DayChart
        lang={lang}
        title={t(lang, "chart.accounts")}
        days={days}
        series={[
          { key: "registered", label: t(lang, "chart.registered"), hue: "c1" },
          { key: "guests", label: t(lang, "chart.guests"), hue: "c2" },
        ]}
      />
    </div>
  );
}

function Details({
  lang,
  stats,
  facets,
  health,
  latency,
}: {
  lang: Lang;
  stats: Stats;
  facets: Facets | null;
  health: Health | null;
  latency: number | null;
}) {
  const n = (v: number) => formatCount(lang, v);
  const onOff = (on: boolean | undefined) =>
    on === undefined ? (
      "—"
    ) : (
      <Badge tone={on ? "ok" : "muted"}>
        {t(lang, on ? "stats.on" : "stats.off")}
      </Badge>
    );
  const cap = !stats.guests.enabled
    ? t(lang, "stats.guestsOff")
    : stats.guests.cap === null || stats.guests.cap === 0
      ? t(lang, "stats.noCap")
      : t(lang, "stats.guestCap", { cap: n(stats.guests.cap) });
  return (
    <div className="panels">
      <Panel id="st-players" title={t(lang, "stats.players")}>
        <p className="big" data-testid="registered">
          <span className="big-number">{n(stats.accounts.registered)}</span>{" "}
          <span className="big-label">{t(lang, "stats.registered")}</span>
        </p>
        <Facts
          rows={[
            [t(lang, "stats.withEmail"), n(stats.accounts.with_email)],
            [t(lang, "stats.confirmed"), n(stats.accounts.confirmed_email)],
            [t(lang, "stats.byKey"), n(stats.accounts.admitted_by_key)],
            [t(lang, "stats.guests"), `${n(stats.guests.live)} · ${cap}`],
          ]}
        />
        <h3>{t(lang, "stats.newAccounts")}</h3>
        <Facts rows={sinceRows(lang, stats.accounts.created)} />
        {stats.guests.created && (
          <>
            <h3>{t(lang, "stats.guestsNew")}</h3>
            <Facts rows={sinceRows(lang, stats.guests.created)} />
          </>
        )}
      </Panel>
      <Panel id="st-online" title={t(lang, "stats.online")}>
        <Facts
          rows={[
            [t(lang, "stats.players"), n(stats.online.players)],
            [t(lang, "stats.inLobby"), n(stats.online.in_lobby)],
            [t(lang, "stats.seated"), n(stats.online.seated)],
            [t(lang, "stats.sessions"), n(stats.online.sessions_live)],
            [t(lang, "stats.signedIn"), n(stats.online.accounts_signed_in)],
          ]}
        />
      </Panel>
      <Panel id="st-games" title={t(lang, "stats.games")}>
        <Facts
          rows={[
            [t(lang, "stats.running"), n(stats.games.running)],
            [t(lang, "stats.local"), n(stats.games.local_running)],
            [t(lang, "stats.waiting"), n(stats.games.waiting)],
            [
              t(lang, "stats.awaitingEngine"),
              n(stats.games.seats_awaiting_engine),
            ],
            ...(stats.games.avg_secs_30d
              ? [
                  [
                    t(lang, "stats.avgGame"),
                    formatDuration(lang, stats.games.avg_secs_30d),
                  ] as [string, string],
                ]
              : []),
          ]}
        />
        <h3>{t(lang, "stats.started")}</h3>
        <Facts rows={sinceRows(lang, stats.games.started)} />
        <h3>{t(lang, "stats.finished")}</h3>
        <Facts
          rows={[
            [t(lang, "stats.total"), n(stats.games.finished)],
            ...sinceRows(lang, stats.games.finished_since),
          ]}
        />
      </Panel>
      <Panel id="st-server" title={t(lang, "stats.server")}>
        <p className="big" data-testid="agents">
          <span className="big-number">{n(stats.agents.connected)}</span>{" "}
          <span className="big-label">{t(lang, "stats.agents")}</span>
        </p>
        <Facts
          rows={[
            [t(lang, "stats.local"), n(stats.agents.local)],
            [
              t(lang, "stats.capacity"),
              stats.agents.capacity === null
                ? t(lang, "stats.unlimited")
                : n(stats.agents.capacity),
            ],
            [t(lang, "stats.agentGames"), n(stats.agents.games)],
            [t(lang, "stats.registration"), stats.gateway.registration],
            [t(lang, "stats.mail"), onOff(stats.gateway.mail)],
            [t(lang, "stats.terms"), onOff(stats.gateway.terms)],
            ...(stats.gateway.uptime_secs === undefined
              ? []
              : [
                  [
                    t(lang, "stats.uptime"),
                    formatDuration(lang, stats.gateway.uptime_secs),
                  ] as [string, string],
                ]),
          ]}
        />
        <h3>{t(lang, "stats.version")}</h3>
        <p className="mono small build">{stats.gateway.version}</p>
      </Panel>
      <Panel id="st-data" title={t(lang, "stats.data")}>
        <Facts
          rows={[
            ...(stats.accounts.decks === undefined
              ? []
              : [
                  [t(lang, "stats.decks"), n(stats.accounts.decks)] as [
                    string,
                    string,
                  ],
                ]),
            [t(lang, "stats.records"), n(stats.games.recorded)],
            ...(stats.games.record_bytes === undefined
              ? []
              : [
                  [
                    t(lang, "stats.recordBytes"),
                    formatBytes(lang, stats.games.record_bytes),
                  ] as [string, string],
                ]),
          ]}
        />
        {facets !== null && facets.statuses.length > 0 && (
          <>
            <h3>{t(lang, "stats.reports")}</h3>
            <Facts
              rows={facets.statuses.map(
                (c) =>
                  [c.value.replace("_", " "), n(c.count)] as [string, string],
              )}
            />
          </>
        )}
      </Panel>
      <Panel
        id="st-keys"
        title={t(lang, "stats.keys")}
        actions={
          <Link to={adminPath("keys")} className="button small-button">
            {t(lang, "sec.keys")} →
          </Link>
        }
      >
        <p className="big" data-testid="keys-active">
          <span className="big-number">{n(stats.invites.active)}</span>{" "}
          <span className="big-label">{t(lang, "stats.activeKeys")}</span>
        </p>
        <Facts
          rows={[
            [t(lang, "stats.usesLeft"), n(stats.invites.uses_left)],
            [t(lang, "stats.admitted"), n(stats.invites.admitted)],
            [t(lang, "stats.usedUp"), n(stats.invites.used_up)],
            [t(lang, "stats.expired"), n(stats.invites.expired)],
            [t(lang, "stats.revoked"), n(stats.invites.revoked)],
          ]}
        />
      </Panel>
      <Services lang={lang} stats={stats} health={health} latency={latency} />
    </div>
  );
}

export function Overview({ lang }: { lang: Lang }) {
  const [stats, setStats] = useState<Stats | null>(null);
  const [facets, setFacets] = useState<Facets | null>(null);
  const [live, setLive] = useState<Live | null>(null);
  const [reports, setReports] = useState<ReportStats | null>(null);
  const [health, setHealth] = useState<Health | null>(null);
  const [latency, setLatency] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    const asked = performance.now();
    api.admin
      .stats()
      .then((s) => {
        setStats(s);
        setLatency(performance.now() - asked);
        setError(null);
      })
      .catch((e: unknown) => {
        setLatency(null);
        if (!(e instanceof SignedOut)) setError(describe(lang, e));
      });
    api
      .stats()
      .then(setReports)
      .catch(() => {
        // The chart of reports is a bonus.
      });
    api
      .health()
      .then(setHealth)
      .catch(() => {
        setHealth({ ok: false, version: "", source: "" });
      });
    api.admin
      .live()
      .then(setLive)
      .catch(() => {
        // The overview reads without the tables.
      });
    api
      .facets()
      .then(setFacets)
      .catch(() => {
        // Nor does it need the reports.
      });
  }, [lang]);

  const shown = useVisibleInterval(load, REFRESH_MS);
  const open = live?.tables ?? [];

  return (
    <>
      <div className="page-head">
        <div>
          <h1>{stats?.gateway.name ?? t(lang, "sec.overview")}</h1>
          <Freshness lang={lang} at={stats?.at ?? null} shown={shown} />
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
      {stats === null && error === null && (
        <div className="skeleton" aria-hidden="true" />
      )}
      {stats !== null && (
        <>
          <Headline lang={lang} stats={stats} facets={facets} live={live} />
          <Charts lang={lang} stats={stats} reports={reports} />
          {open.length > 0 && (
            <Panel
              id="st-live"
              title={t(lang, "live.tables")}
              className="wide-panel"
              actions={
                <Link to={adminPath("live")} className="button small-button">
                  {t(lang, "sec.live")} →
                </Link>
              }
            >
              <div className="table-cards">
                {open.slice(0, 4).map((table) => (
                  <TableCard key={table.id} lang={lang} table={table} compact />
                ))}
              </div>
            </Panel>
          )}
          <Details
            lang={lang}
            stats={stats}
            facets={facets}
            health={health}
            latency={latency}
          />
        </>
      )}
    </>
  );
}
