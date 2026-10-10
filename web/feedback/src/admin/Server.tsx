// The server (`/ui/api/admin/metrics`, docs/protocol.md §"The admin
// console"): what the gateway's machine is doing, as tiles with the last
// hour under each as a small line, and every engine process with its CPU
// and memory. A field the host could not say (anything off `/proc` on a
// developer's Mac) is a dash, never a zero.

import { useId } from "react";

import type { GameSample, Metrics } from "../api";
import {
  formatBytes,
  formatCount,
  formatDuration,
  t,
  type Lang,
} from "../i18n";
import { useWidth } from "./Chart";
import { Panel } from "./shared";

const HEIGHT = 36;

/** A number as a percentage with no decimals, or a dash. */
function percent(lang: Lang, share: number | null): string {
  if (share === null) return "—";
  return `${new Intl.NumberFormat(lang === "de" ? "de-DE" : "en-GB", { maximumFractionDigits: 0 }).format(share * 100)} %`;
}

/** A rate of bytes a second, or a dash. */
function rate(lang: Lang, bytesPerSecond: number | null): string {
  if (bytesPerSecond === null) return "—";
  return `${formatBytes(lang, bytesPerSecond)}/s`;
}

/** A float with up to `digits` decimals, or a dash. */
function fixed(lang: Lang, n: number | null, digits = 2): string {
  if (n === null) return "—";
  return new Intl.NumberFormat(lang === "de" ? "de-DE" : "en-GB", {
    maximumFractionDigits: digits,
  }).format(n);
}

/**
 * The last hour as one line, missing points left out. `max` fixes the top
 * (a share of 1, a known total); otherwise the line is scaled to its peak.
 */
export function Sparkline({
  values,
  max,
  label,
  hue = "c1",
}: {
  values: (number | null)[];
  max?: number | undefined;
  label: string;
  hue?: "c1" | "c2";
}) {
  const [ref, width] = useWidth();
  const known = values.filter((v): v is number => v !== null);
  const top = max ?? Math.max(1e-9, ...known);
  const n = values.length;
  const points = values
    .map((v, i) => {
      if (v === null) return null;
      const x = n <= 1 ? width : (i / (n - 1)) * width;
      const y = HEIGHT - Math.min(1, Math.max(0, v / top)) * (HEIGHT - 2) - 1;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .filter((p): p is string => p !== null);
  return (
    <div className="spark" ref={ref}>
      <svg width={width} height={HEIGHT} focusable="false" aria-label={label}>
        <title>{label}</title>
        {points.length > 1 && (
          <polyline className={`spark-line ${hue}`} points={points.join(" ")} />
        )}
        {points.length === 1 && (
          <circle
            className={`spark-dot ${hue}`}
            r={2}
            cx={width}
            cy={points[0]?.split(",")[1]}
          />
        )}
      </svg>
    </div>
  );
}

function Tile({
  label,
  value,
  sub,
  spark,
  max,
  sparkLabel,
  testId,
}: {
  label: string;
  value: string;
  sub?: string | undefined;
  spark: (number | null)[];
  max?: number | undefined;
  sparkLabel: string;
  testId: string;
}) {
  return (
    <div className="metric" data-testid={testId}>
      <span className="metric-label">{label}</span>
      <span className="metric-value">{value}</span>
      {sub !== undefined && (
        <span className="metric-sub muted small">{sub}</span>
      )}
      <Sparkline values={spark} max={max} label={sparkLabel} />
    </div>
  );
}

/** A table's engine process: CPU and memory with its line. */
function GameRow({
  lang,
  game,
  metrics,
}: {
  lang: Lang;
  game: GameSample;
  metrics: Metrics;
}) {
  const history = metrics.games.find(
    (g) => g.game_id === game.game_id && g.pid === game.pid,
  );
  return (
    <li className="metric-game" data-testid="engine-process">
      <span className="metric-game-id">
        <a href={`#table-${game.game_id}`}>{game.game_id}</a>
        <span className="muted small"> · pid {game.pid}</span>
      </span>
      <span className="metric-game-num">
        {t(lang, "srv.cores", { n: fixed(lang, game.cpu) })}
      </span>
      <span className="metric-game-num">{formatBytes(lang, game.rss)}</span>
      <Sparkline
        values={history?.cpu ?? [game.cpu]}
        max={Math.max(
          1,
          ...(history?.cpu ?? []).filter((v): v is number => v !== null),
        )}
        label={t(lang, "srv.gameSpark", { id: game.game_id })}
        hue="c2"
      />
    </li>
  );
}

export function ServerPanel({
  lang,
  metrics,
}: {
  lang: Lang;
  metrics: Metrics;
}) {
  const id = useId();
  const now = metrics.now;
  const h = metrics.history;
  const n = (v: number) => formatCount(lang, v);
  const memTotal = metrics.host.mem_total;
  const diskTotal = metrics.host.disk_total;
  const minutes = Math.round((h.at.length * metrics.interval_secs) / 60);
  const span =
    minutes >= 60
      ? t(lang, "srv.lastHour")
      : t(lang, "srv.lastMinutes", { n: n(Math.max(1, minutes)) });
  const procsLine = (() => {
    if (now === null) return null;
    const running = now.games.length;
    const unseen = metrics.platform === "linux" ? null : t(lang, "srv.noProc");
    return running === 0 ? (unseen ?? t(lang, "srv.noGames")) : null;
  })();
  return (
    <Panel
      id={`${id}-server`}
      title={t(lang, "srv.title")}
      className="live-server"
      actions={
        <span className="muted small">
          {metrics.platform}
          {" · "}
          {t(lang, "srv.coresN", { n: n(metrics.cores) })}
          {" · "}
          {span}
        </span>
      }
    >
      {now === null ? (
        <p className="empty">{t(lang, "srv.noSample")}</p>
      ) : (
        <>
          <div className="metrics">
            <Tile
              testId="m-cpu"
              label={t(lang, "srv.cpu")}
              value={percent(lang, now.cpu)}
              sub={
                metrics.host.load === null
                  ? undefined
                  : t(lang, "srv.load", {
                      l: metrics.host.load
                        .map((l) => fixed(lang, l))
                        .join(" · "),
                    })
              }
              spark={h.cpu}
              max={1}
              sparkLabel={t(lang, "srv.sparkOf", { what: t(lang, "srv.cpu") })}
            />
            <Tile
              testId="m-mem"
              label={t(lang, "srv.memory")}
              value={
                now.mem_used === null ? "—" : formatBytes(lang, now.mem_used)
              }
              sub={
                memTotal === null
                  ? undefined
                  : t(lang, "srv.ofTotal", {
                      total: formatBytes(lang, memTotal),
                    })
              }
              spark={h.mem_used}
              max={memTotal ?? undefined}
              sparkLabel={t(lang, "srv.sparkOf", {
                what: t(lang, "srv.memory"),
              })}
            />
            <Tile
              testId="m-disk"
              label={t(lang, "srv.disk")}
              value={
                now.disk_used === null ? "—" : formatBytes(lang, now.disk_used)
              }
              sub={
                diskTotal === null
                  ? undefined
                  : `${t(lang, "srv.ofTotal", { total: formatBytes(lang, diskTotal) })} · ${metrics.host.disk_path}`
              }
              spark={h.disk_used}
              max={diskTotal ?? undefined}
              sparkLabel={t(lang, "srv.sparkOf", { what: t(lang, "srv.disk") })}
            />
            <Tile
              testId="m-net"
              label={t(lang, "srv.network")}
              value={`↓ ${rate(lang, now.net_in)}`}
              sub={`↑ ${rate(lang, now.net_out)}`}
              spark={h.net_in}
              sparkLabel={t(lang, "srv.sparkOf", {
                what: t(lang, "srv.network"),
              })}
            />
            <Tile
              testId="m-req"
              label={t(lang, "srv.requests")}
              value={t(lang, "srv.perSecond", {
                n: fixed(lang, now.requests, 1),
              })}
              sub={
                now.sockets === null
                  ? undefined
                  : t(lang, "srv.sockets", { n: n(now.sockets) })
              }
              spark={h.requests}
              sparkLabel={t(lang, "srv.sparkOf", {
                what: t(lang, "srv.requests"),
              })}
            />
            <Tile
              testId="m-up"
              label={t(lang, "srv.uptime")}
              value={formatDuration(lang, metrics.process_uptime_secs)}
              sub={
                metrics.host.uptime_secs === null
                  ? t(lang, "srv.gatewayProcess")
                  : t(lang, "srv.hostUp", {
                      d: formatDuration(lang, metrics.host.uptime_secs),
                    })
              }
              spark={h.sockets}
              sparkLabel={t(lang, "srv.sparkOf", {
                what: t(lang, "srv.socketsLabel"),
              })}
            />
          </div>
          <h3>{t(lang, "srv.engines")}</h3>
          {procsLine !== null ? (
            <p className="empty">{procsLine}</p>
          ) : (
            <ul className="metric-games">
              {now.games.map((game) => (
                <GameRow
                  key={`${game.game_id}-${game.pid}`}
                  lang={lang}
                  game={game}
                  metrics={metrics}
                />
              ))}
            </ul>
          )}
        </>
      )}
    </Panel>
  );
}
