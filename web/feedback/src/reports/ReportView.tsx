// One report, read: what the player wrote with its references as chips,
// where in the game they were, the crash, the screenshot, the log, the
// record, and the triage beside it. The page (`/r/{id}`) and the list's
// drawer draw the same thing; the drawer in its compact form, with the
// large parts folded.

import { useCallback, useEffect, useState, type FormEvent, type ReactNode } from "react";

import {
  api,
  isRecord,
  KINDS,
  Refused,
  SignedOut,
  STATUSES,
  type AuditEntry,
  type Kind,
  type Report,
  type Status,
} from "../api";
import { part, readBuild, readCrash, readLog, readRefs, readScreenshot, readSystem } from "../dump";
import { formatBytes } from "../format";
import { CATEGORY_TITLES, newIssueUrl, parseIssue } from "../github";
import { kindLabel, statusLabel, t, type Lang } from "../i18n";
import { JsonTree } from "../JsonTree";
import { pendingLabel, readGame, type GameSummary } from "./game";
import { CardChip, PlayerChip, RefText, Reporter, Time, TimeBoth, toast } from "./pieces";
import { navigate } from "../router";

export function Section({
  title,
  children,
  id,
  folded = false,
}: {
  title: string;
  children: ReactNode;
  id: string;
  /** Drawn as a closed `<details>`: for the large parts in the drawer. */
  folded?: boolean;
}) {
  if (folded) {
    return (
      <details className="card folded">
        <summary>
          <h2 id={id}>{title}</h2>
        </summary>
        {children}
      </details>
    );
  }
  return (
    <section className="card" aria-labelledby={id}>
      <h2 id={id}>{title}</h2>
      {children}
    </section>
  );
}

function Missing({ lang, what }: { lang: Lang; what: string }) {
  return <p className="muted">{t(lang, "rep.missing", { what })}</p>;
}

export function describe(lang: Lang, e: unknown): string {
  return e instanceof Refused ? e.message : t(lang, "admin.noAnswer");
}

/**
 * The way to a new GitHub issue: only the admin's words, the category and
 * the build, nothing of the report itself (owner, 08.10.2026;
 * `github.ts`). Its own state, so typing re-renders this form alone.
 */
function NewIssue({ lang, kind, build, rows }: { lang: Lang; kind: Kind; build: string; rows: number }) {
  const [summary, setSummary] = useState("");
  const [category, setCategory] = useState<Kind | null>(null);
  const url = newIssueUrl({ summary, category: category ?? kind, build });
  return (
    <div className="new-issue">
      <label htmlFor="d-issue-summary">{t(lang, "rep.summaryLabel")}</label>
      <p className="muted small" id="d-issue-hint">
        {t(lang, "rep.summaryHint")}
      </p>
      <textarea
        id="d-issue-summary"
        rows={rows}
        aria-describedby="d-issue-hint"
        value={summary}
        onChange={(event) => {
          setSummary(event.target.value);
        }}
      />
      <label htmlFor="d-issue-category">{t(lang, "rep.category")}</label>
      <select
        id="d-issue-category"
        value={category ?? kind}
        onChange={(event) => {
          const chosen = KINDS.find((k) => k === event.target.value);
          if (chosen) setCategory(chosen);
        }}
      >
        {KINDS.map((k) => (
          <option key={k} value={k}>
            {CATEGORY_TITLES[k]}
          </option>
        ))}
      </select>
      <p>
        {url === null ? (
          <button type="button" disabled>
            {t(lang, "rep.openIssue")}
          </button>
        ) : (
          <a className="button" href={url} target="_blank" rel="noopener noreferrer">
            {t(lang, "rep.openIssue")}
          </a>
        )}
      </p>
    </div>
  );
}

/** Where in the game the report was written, in a few lines. */
export function TableSummary({ lang, game }: { lang: Lang; game: GameSummary }) {
  const rows: [string, ReactNode][] = [];
  if (game.turn !== null) {
    const who =
      game.activeSeat === null
        ? ""
        : game.activeSeat === game.seat
          ? t(lang, "rep.ownTurn")
          : t(lang, "rep.seatsTurn", { n: game.activeSeat });
    rows.push([t(lang, "rep.turn"), who ? t(lang, "rep.turnOf", { n: game.turn, who }) : String(game.turn)]);
  }
  if (game.when !== null) rows.push([t(lang, "rep.phase"), game.when]);
  else if (game.phase !== null) rows.push([t(lang, "rep.phase"), `${game.phase}${game.step !== null ? ` · ${game.step}` : ""}`]);
  if (game.seat !== null) {
    rows.push([
      t(lang, "rep.seat"),
      `${game.seat}${game.players !== null ? ` · ${t(lang, "rep.players", { n: game.players })}` : ""}`,
    ]);
  }
  rows.push([
    t(lang, "rep.pending"),
    game.pending === null ? (
      <span className="muted" key="none">
        {t(lang, "rep.noPending")}
      </span>
    ) : (
      <span key="pending">
        <strong>{pendingLabel(game.pending)}</strong>
        {game.pendingDetail !== null && <span className="muted small"> · {game.pendingDetail}</span>}
        {game.awaiting !== null && <span className="muted small"> · {t(lang, "rep.awaiting", { n: game.awaiting })}</span>}
      </span>
    ),
  ]);
  if (game.holding) {
    const bits: string[] = [];
    if (game.holding.armed !== null) bits.push(t(lang, "rep.armed", { what: game.holding.armed }));
    if (game.holding.selected !== null && game.holding.selected > 0) bits.push(t(lang, "rep.selectedN", { n: game.holding.selected }));
    if (game.holding.outbox !== null && game.holding.outbox > 0) bits.push(t(lang, "rep.outbox", { n: game.holding.outbox }));
    if (game.holding.manaRun) bits.push(t(lang, "rep.manaRun"));
    if (bits.length > 0) rows.push([t(lang, "rep.holding"), bits.join(" · ")]);
    if (game.holding.lastError !== null) rows.push([t(lang, "rep.lastError"), <code key="e">{game.holding.lastError}</code>]);
  }
  if (game.seq !== null) rows.push([t(lang, "rep.view"), t(lang, "rep.viewSeq", { n: game.seq })]);
  return (
    <dl className="facts">
      {rows.map(([term, value]) => (
        <div key={term} className="fact">
          <dt>{term}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

/** A plain-text summary for the clipboard: the facts and the text, nothing from the client object. */
export function summaryText(report: Report, lang: Lang): string {
  const build = readBuild(report.client);
  return [
    `${kindLabel(lang, report.kind)} · ${statusLabel(lang, report.status)} · ${report.id}`,
    `${t(lang, "rep.received")}: ${report.created_at}`,
    `${t(lang, "rep.gateway")}: ${report.channel === "direct" ? "direct" : report.gateway} · ${report.gateway_version}`,
    `${t(lang, "rep.clientBuild")}: ${build ? `${build.version ?? "?"}${build.commit ? ` (${build.commit})` : ""}` : "—"}`,
    `${t(lang, "rep.game")}: ${report.game_id ?? "—"}`,
    report.issue_number !== null ? `${t(lang, "rep.issue")}: #${report.issue_number}` : "",
    "",
    report.text,
  ]
    .filter((line, i, all) => line !== "" || i === all.length - 2)
    .join("\n");
}

export function ReportView({
  id,
  lang,
  compact = false,
  onChanged,
  onDeleted,
}: {
  id: string;
  lang: Lang;
  compact?: boolean;
  onChanged?: (report: Report) => void;
  onDeleted?: (id: string) => void;
}) {
  const [report, setReport] = useState<Report | null>(null);
  const [audit, setAudit] = useState<AuditEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [issueDraft, setIssueDraft] = useState("");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [busy, setBusy] = useState(false);

  const refreshAudit = useCallback(() => {
    api
      .audit(id)
      .then(setAudit)
      .catch(() => {
        // The audit is a footnote; the report reads without it.
      });
  }, [id]);

  useEffect(() => {
    let live = true;
    api
      .report(id)
      .then((r) => {
        if (live) setReport(r);
      })
      .catch((e: unknown) => {
        if (live && !(e instanceof SignedOut)) setError(describe(lang, e));
      });
    refreshAudit();
    return () => {
      live = false;
    };
  }, [id, refreshAudit, lang]);

  const change = async (what: { status?: Status; issue?: number | null }, done: string) => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const updated = await api.change(id, what);
      setReport(updated);
      setNotice(done);
      onChanged?.(updated);
      refreshAudit();
    } catch (e: unknown) {
      if (!(e instanceof SignedOut)) setError(describe(lang, e));
    } finally {
      setBusy(false);
    }
  };

  const link = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const number = parseIssue(issueDraft);
    if (number === null) {
      setError(t(lang, "rep.badIssue"));
      return;
    }
    setIssueDraft("");
    void change({ issue: number }, t(lang, "rep.linked", { n: number }));
  };

  const remove = async () => {
    setBusy(true);
    try {
      await api.remove(id);
      toast("ok", t(lang, "rep.deleted"));
      if (onDeleted) onDeleted(id);
      else navigate("/", true);
    } catch (e: unknown) {
      if (!(e instanceof SignedOut)) setError(describe(lang, e));
      setBusy(false);
    }
  };

  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast("ok", t(lang, "rep.copied"));
    } catch {
      toast("error", t(lang, "rep.copyFailed"));
    }
  };

  if (report === null) {
    return error !== null ? (
      <p className="error" role="alert">
        {error}
      </p>
    ) : (
      <p className="muted">{t(lang, "rep.loading")}</p>
    );
  }

  // A report from before the client object, or a stand-in in a test, reads
  // as one that sent nothing.
  const client = isRecord(report.client) ? report.client : {};
  const build = readBuild(client);
  const system = readSystem(client);
  const screenshot = readScreenshot(client);
  const log = readLog(client);
  const crash = readCrash(client);
  const refs = readRefs(client);
  const game = readGame(client);
  const gameJson = part(client, "game");
  const settings = part(client, "settings");
  const pageLink = `${window.location.origin}/r/${report.id}`;

  const triage = (
    <Section title={t(lang, "rep.triage")} id="s-triage">
      <label htmlFor="d-status">{t(lang, "rep.status")}</label>
      <select
        id="d-status"
        value={report.status}
        disabled={busy}
        onChange={(event) => {
          const status = STATUSES.find((s) => s === event.target.value);
          if (status) void change({ status }, t(lang, "rep.statusSet", { status: statusLabel(lang, status) }));
        }}
      >
        {STATUSES.map((s) => (
          <option key={s} value={s}>
            {statusLabel(lang, s)}
          </option>
        ))}
      </select>
      <fieldset className="quick-status">
        <legend className="sr-only">{t(lang, "rep.setStatus")}</legend>
        {STATUSES.map((s, i) => (
          <button
            key={s}
            type="button"
            className={`chip-button${report.status === s ? " on" : ""}`}
            disabled={busy || report.status === s}
            title={`${i + 1}`}
            onClick={() => void change({ status: s }, t(lang, "rep.statusSet", { status: statusLabel(lang, s) }))}
          >
            {statusLabel(lang, s)}
          </button>
        ))}
      </fieldset>

      <h3>{t(lang, "rep.issue")}</h3>
      {report.issue_number !== null && report.issue_url !== null ? (
        <p>
          <a href={report.issue_url} target="_blank" rel="noopener noreferrer">
            #{report.issue_number}
          </a>{" "}
          <button
            type="button"
            className="quiet"
            disabled={busy}
            onClick={() => void change({ issue: null }, t(lang, "rep.unlinked"))}
          >
            {t(lang, "rep.unlink")}
          </button>
        </p>
      ) : (
        <p className="muted">{t(lang, "rep.notLinked")}</p>
      )}
      <form className="inline" onSubmit={link}>
        <label htmlFor="d-issue" className="sr-only">
          {t(lang, "rep.issueLabel")}
        </label>
        <input
          id="d-issue"
          placeholder={t(lang, "rep.issuePlaceholder")}
          value={issueDraft}
          onChange={(event) => {
            setIssueDraft(event.target.value);
          }}
        />
        <button type="submit" disabled={busy || issueDraft.trim() === ""}>
          {t(lang, "rep.link")}
        </button>
      </form>
      <NewIssue lang={lang} kind={report.kind} build={report.gateway_version} rows={compact ? 3 : 4} />

      <div className="row-actions">
        <button type="button" className="quiet" onClick={() => void copy(pageLink)}>
          {t(lang, "rep.copyLink")}
        </button>
        <button type="button" className="quiet" onClick={() => void copy(report.id)}>
          {t(lang, "rep.copyId")}
        </button>
        <button type="button" className="quiet" onClick={() => void copy(summaryText(report, lang))}>
          {t(lang, "rep.copySummary")}
        </button>
      </div>

      {notice !== null && <output className="notice">{notice}</output>}
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </Section>
  );

  const record = (
    <Section title={t(lang, "rep.recordTitle")} id="s-record">
      {report.has_record ? (
        <>
          <p>
            {formatBytes(report.record_bytes)}, {report.record_complete ? t(lang, "rep.complete") : t(lang, "rep.partial")}
            {report.record_origin === "client" && (
              <>
                {" "}
                <span className="tag">{t(lang, "rep.clientRecord")}</span>
              </>
            )}
          </p>
          <a className="button" href={api.recordUrl(report.id)} download={`${report.id}.jsonl.gz`}>
            {t(lang, "rep.download")}
          </a>
          {!compact && <p className="muted small">{t(lang, "rep.recordHint")}</p>}
        </>
      ) : (
        <p className="muted">{t(lang, "rep.noRecord")}</p>
      )}
    </Section>
  );

  const facts = (
    <dl className="facts">
      <dt>{t(lang, "rep.received")}</dt>
      <dd>
        <TimeBoth lang={lang} iso={report.created_at} />
      </dd>
      <dt>{t(lang, "rep.changed")}</dt>
      <dd>
        <Time lang={lang} iso={report.updated_at} />
      </dd>
      <dt>{t(lang, "rep.gateway")}</dt>
      <dd>
        {report.channel === "direct" ? <span className="tag">{t(lang, "rep.direct")}</span> : report.gateway}
        {report.gateway_name !== null && <span className="muted"> · {report.gateway_name}</span>}
        {!compact && report.gateway_url !== null && <span className="muted"> · {report.gateway_url}</span>}
      </dd>
      <dt>{t(lang, "rep.gatewayBuild")}</dt>
      <dd className="mono">{report.gateway_version}</dd>
      <dt>{t(lang, "rep.clientBuild")}</dt>
      <dd className="mono">
        {build ? `${build.version ?? "?"}${build.commit ? ` (${build.commit})` : ""}` : "—"}
      </dd>
      {system !== null && system.length > 0 && compact && (
        <>
          <dt>{t(lang, "rep.platform")}</dt>
          <dd>{system.find(([label]) => label === "Platform")?.[1] ?? "—"}</dd>
        </>
      )}
      <dt>{t(lang, "rep.reporter")}</dt>
      <dd>
        <Reporter
          lang={lang}
          pseudonym={report.reporter}
          onClick={() => {
            navigate(`/?reporter=${encodeURIComponent(report.reporter)}`);
          }}
        />
      </dd>
      <dt>{t(lang, "rep.game")}</dt>
      <dd className="mono">{report.game_id ?? "—"}</dd>
    </dl>
  );

  const main = (
    <>
      <Section title={t(lang, "rep.wrote")} id="s-text">
        {report.text.trim() === "" ? (
          <p className="muted">{t(lang, "rep.noText")}</p>
        ) : (
          <p className="report-text">
            <RefText text={report.text} refs={refs} lang={lang} />
          </p>
        )}
      </Section>

      {refs !== null && (
        <Section title={t(lang, "rep.refs")} id="s-refs">
          <ul className="refs">
            {refs.cards.map((card, i) => (
              <li key={`c${i}`}>
                <CardChip lang={lang} name={card.text} scryfallId={card.scryfallId} />
                {card.zone !== null && <span className="muted"> · {card.zone.replace("_", " ")}</span>}
                {card.owner !== null && <span className="muted"> · {t(lang, "rep.seatN", { n: card.owner })}</span>}
                {card.card !== null && <span className="muted mono"> · {t(lang, "rep.card", { n: card.card })}</span>}
                {card.scryfall !== null && (
                  <>
                    {" · "}
                    <a href={card.scryfall} target="_blank" rel="noopener noreferrer">
                      Scryfall
                    </a>
                  </>
                )}
              </li>
            ))}
            {refs.players.map((player, i) => (
              <li key={`p${i}`}>
                <PlayerChip lang={lang} name={player.text} seat={player.seat} />
                {player.seat !== null && <span className="muted"> · {t(lang, "rep.seatN", { n: player.seat })}</span>}
              </li>
            ))}
          </ul>
        </Section>
      )}

      <Section title={t(lang, "rep.table")} id="s-table">
        {game ? <TableSummary lang={lang} game={game} /> : <p className="muted">{t(lang, "rep.noTable")}</p>}
      </Section>

      {crash !== null && (
        <Section title={t(lang, "rep.crash")} id="s-crash">
          <p className="mono">{crash.message}</p>
          <dl className="facts">
            {crash.location !== null && (
              <>
                <dt>{t(lang, "rep.where")}</dt>
                <dd className="mono">{crash.location}</dd>
              </>
            )}
            {crash.thread !== null && (
              <>
                <dt>{t(lang, "rep.thread")}</dt>
                <dd className="mono">{crash.thread}</dd>
              </>
            )}
            {crash.platform !== null && (
              <>
                <dt>{t(lang, "rep.platform")}</dt>
                <dd>{crash.platform}</dd>
              </>
            )}
            {crash.at !== null && (
              <>
                <dt>{t(lang, "rep.when")}</dt>
                <dd>
                  <TimeBoth lang={lang} iso={crash.at} />
                </dd>
              </>
            )}
          </dl>
          {crash.backtrace !== null && (
            <details>
              <summary>{t(lang, "rep.backtrace")}</summary>
              <pre className="pre">{crash.backtrace}</pre>
            </details>
          )}
        </Section>
      )}

      <Section title={t(lang, "rep.screenshot")} id="s-shot">
        {screenshot ? (
          <a href={screenshot.src} target="_blank" rel="noopener noreferrer">
            <img
              className="screenshot"
              src={screenshot.src}
              alt={t(lang, "rep.screenshotAlt")}
              width={screenshot.width ?? undefined}
              height={screenshot.height ?? undefined}
            />
          </a>
        ) : (
          <Missing lang={lang} what={t(lang, "rep.aScreenshot")} />
        )}
      </Section>

      <Section title={t(lang, "rep.log")} id="s-log" folded={compact}>
        {log ? (
          <>
            {log.roster.length > 0 && (
              <p className="muted">
                {log.roster
                  .map((r) => `${r.name}${r.isAi ? " (AI)" : ""}${r.team !== null ? `, team ${r.team}` : ""}`)
                  .join(" · ")}
              </p>
            )}
            {log.lines.length === 0 ? (
              <p className="muted">{t(lang, "rep.logEmpty")}</p>
            ) : (
              <ol className="log">
                {log.lines.map((line, i) => (
                  <li key={i}>
                    <span className="muted mono">
                      {line.turn !== null ? `T${line.turn}` : ""} {line.clock}
                    </span>{" "}
                    {line.text}
                  </li>
                ))}
              </ol>
            )}
          </>
        ) : (
          <Missing lang={lang} what={t(lang, "rep.theLog")} />
        )}
      </Section>

      <Section title={t(lang, "rep.tableJson")} id="s-game" folded={compact}>
        {gameJson ? <JsonTree value={gameJson} open={1} /> : <Missing lang={lang} what={t(lang, "rep.theTable")} />}
      </Section>

      <Section title={t(lang, "rep.everything")} id="s-client" folded={compact}>
        <JsonTree value={client} open={compact ? 0 : 1} />
      </Section>
    </>
  );

  const rest = (
    <>
      <Section title={t(lang, "rep.system")} id="s-system" folded={compact}>
        {system && system.length > 0 ? (
          <dl className="facts">
            {system.map(([label, value]) => (
              <div key={label} className="fact">
                <dt>{label}</dt>
                <dd>{value}</dd>
              </div>
            ))}
          </dl>
        ) : (
          <Missing lang={lang} what={t(lang, "rep.systemDetails")} />
        )}
      </Section>
      <Section title={t(lang, "rep.settings")} id="s-settings" folded={compact}>
        {settings ? <JsonTree value={settings} open={1} /> : <Missing lang={lang} what={t(lang, "rep.theSettings")} />}
      </Section>
      <Section title={t(lang, "rep.history")} id="s-audit" folded={compact}>
        {audit.length === 0 ? (
          <p className="muted">{t(lang, "rep.noHistory")}</p>
        ) : (
          <ol className="audit">
            {audit.map((entry, i) => (
              <li key={i}>
                <Time lang={lang} iso={entry.at} /> {entry.actor}: {entry.action}
                {entry.detail !== null ? ` ${entry.detail}` : entry.action === "issue" ? " unlinked" : ""}
              </li>
            ))}
          </ol>
        )}
      </Section>
      <Section title={t(lang, "rep.delete").replace("…", "")} id="s-delete">
        {confirmDelete ? (
          <div className="confirm">
            <p>{t(lang, "rep.deleteAsk")}</p>
            <button type="button" className="danger" disabled={busy} onClick={() => void remove()}>
              {t(lang, "rep.deleteYes")}
            </button>{" "}
            <button
              type="button"
              className="quiet"
              onClick={() => {
                setConfirmDelete(false);
              }}
            >
              {t(lang, "rep.deleteNo")}
            </button>
          </div>
        ) : (
          <button
            type="button"
            className="danger-quiet"
            onClick={() => {
              setConfirmDelete(true);
            }}
          >
            {t(lang, "rep.delete")}
          </button>
        )}
      </Section>
    </>
  );

  return (
    <>
      <header className="detail-head">
        <h1>
          <span className={`tag kind-${report.kind}`}>{kindLabel(lang, report.kind)}</span>{" "}
          <span className={`tag status-${report.status}`}>{statusLabel(lang, report.status)}</span>{" "}
          {!compact && <span className="mono small">{report.id}</span>}
        </h1>
        {facts}
      </header>
      {compact ? (
        <div className="drawer-body">
          {triage}
          {main}
          {record}
          {rest}
        </div>
      ) : (
        <div className="detail-grid">
          <div className="detail-main">{main}</div>
          <aside className="detail-side">
            {triage}
            {record}
            {rest}
          </aside>
        </div>
      )}
    </>
  );
}
