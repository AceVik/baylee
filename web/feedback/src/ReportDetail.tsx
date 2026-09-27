import { useCallback, useEffect, useState, type FormEvent, type ReactNode } from "react";

import { api, Refused, SignedOut, STATUS_LABELS, STATUSES, type AuditEntry, type Report, type Status } from "./api";
import { part, readBuild, readCrash, readLog, readScreenshot, readSystem } from "./dump";
import { formatBytes, formatTime } from "./format";
import { newIssueUrl, parseIssue } from "./github";
import { JsonTree } from "./JsonTree";
import { listPath, navigate } from "./router";

function Section({ title, children, id }: { title: string; children: ReactNode; id: string }) {
  return (
    <section className="card" aria-labelledby={id}>
      <h2 id={id}>{title}</h2>
      {children}
    </section>
  );
}

function Missing({ what }: { what: string }) {
  return <p className="muted">The player did not include {what}.</p>;
}

function describe(e: unknown): string {
  return e instanceof Refused ? e.message : "The service did not answer.";
}

export function ReportDetail({ id }: { id: string }) {
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
        if (live && !(e instanceof SignedOut)) setError(describe(e));
      });
    refreshAudit();
    return () => {
      live = false;
    };
  }, [id, refreshAudit]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target;
      const typing =
        target instanceof HTMLElement && ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName);
      if (event.key === "Escape" && !typing) {
        event.preventDefault();
        navigate(listPath());
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  const change = async (what: { status?: Status; issue?: number | null }, done: string) => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const updated = await api.change(id, what);
      setReport(updated);
      setNotice(done);
      refreshAudit();
    } catch (e: unknown) {
      if (!(e instanceof SignedOut)) setError(describe(e));
    } finally {
      setBusy(false);
    }
  };

  const link = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const number = parseIssue(issueDraft);
    if (number === null) {
      setError("Give an issue number (311, #311) or its URL in AceVik/baylee.");
      return;
    }
    setIssueDraft("");
    void change({ issue: number }, `Linked to #${number}.`);
  };

  const remove = async () => {
    setBusy(true);
    try {
      await api.remove(id);
      navigate("/", true);
    } catch (e: unknown) {
      if (!(e instanceof SignedOut)) setError(describe(e));
      setBusy(false);
    }
  };

  if (report === null) {
    return (
      <main className="detail-page">
        <BackLink />
        {error !== null ? (
          <p className="error" role="alert">
            {error}
          </p>
        ) : (
          <p className="muted">Loading…</p>
        )}
      </main>
    );
  }

  const client = report.client;
  const build = readBuild(client);
  const system = readSystem(client);
  const screenshot = readScreenshot(client);
  const log = readLog(client);
  const crash = readCrash(client);
  const game = part(client, "game");
  const settings = part(client, "settings");

  return (
    <main className="detail-page">
      <BackLink />
      <header className="detail-head">
        <h1>
          <span className={`tag kind-${report.kind}`}>{report.kind}</span>{" "}
          <span className="mono small">{report.id}</span>
        </h1>
        <dl className="facts">
          <dt>Received</dt>
          <dd>{formatTime(report.created_at)}</dd>
          <dt>Changed</dt>
          <dd>{formatTime(report.updated_at)}</dd>
          <dt>Gateway</dt>
          <dd>
            {report.gateway}
            {report.gateway_name !== null && <span className="muted"> · {report.gateway_name}</span>}
            {report.gateway_url !== null && <span className="muted"> · {report.gateway_url}</span>}
          </dd>
          <dt>Gateway build</dt>
          <dd className="mono">{report.gateway_version}</dd>
          <dt>Client build</dt>
          <dd className="mono">
            {build ? `${build.version ?? "?"}${build.commit ? ` (${build.commit})` : ""}` : "—"}
          </dd>
          <dt>Reporter</dt>
          <dd>
            <a
              className="mono"
              href={`/?reporter=${encodeURIComponent(report.reporter)}`}
              onClick={(event) => {
                event.preventDefault();
                navigate(`/?reporter=${encodeURIComponent(report.reporter)}`);
              }}
              title="Every report of this pseudonym"
            >
              {report.reporter}
            </a>
          </dd>
          <dt>Game</dt>
          <dd className="mono">{report.game_id ?? "—"}</dd>
        </dl>
      </header>

      <div className="detail-grid">
        <div className="detail-main">
          <Section title="What the player wrote" id="s-text">
            {report.text.trim() === "" ? (
              <p className="muted">(no text)</p>
            ) : (
              <p className="report-text">{report.text}</p>
            )}
          </Section>

          {crash !== null && (
            <Section title="Crash" id="s-crash">
              <p className="mono">{crash.message}</p>
              <dl className="facts">
                {crash.location !== null && (
                  <>
                    <dt>Where</dt>
                    <dd className="mono">{crash.location}</dd>
                  </>
                )}
                {crash.thread !== null && (
                  <>
                    <dt>Thread</dt>
                    <dd className="mono">{crash.thread}</dd>
                  </>
                )}
                {crash.platform !== null && (
                  <>
                    <dt>Platform</dt>
                    <dd>{crash.platform}</dd>
                  </>
                )}
                {crash.at !== null && (
                  <>
                    <dt>When</dt>
                    <dd>{formatTime(crash.at)}</dd>
                  </>
                )}
              </dl>
              {crash.backtrace !== null && (
                <details>
                  <summary>Backtrace</summary>
                  <pre className="pre">{crash.backtrace}</pre>
                </details>
              )}
            </Section>
          )}

          <Section title="Screenshot" id="s-shot">
            {screenshot ? (
              <a href={screenshot.src} target="_blank" rel="noopener noreferrer">
                <img
                  className="screenshot"
                  src={screenshot.src}
                  alt="The player's window when the report was written"
                  width={screenshot.width ?? undefined}
                  height={screenshot.height ?? undefined}
                />
              </a>
            ) : (
              <Missing what="a screenshot" />
            )}
          </Section>

          <Section title="The player's game log" id="s-log">
            {log ? (
              <>
                {log.roster.length > 0 && (
                  <p className="muted">
                    {log.roster
                      .map(
                        (r) =>
                          `${r.name}${r.isAi ? " (AI)" : ""}${r.team !== null ? `, team ${r.team}` : ""}`,
                      )
                      .join(" · ")}
                  </p>
                )}
                {log.lines.length === 0 ? (
                  <p className="muted">The log is empty.</p>
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
              <Missing what="the game log" />
            )}
          </Section>

          <Section title="The table as the player saw it" id="s-game">
            {game ? <JsonTree value={game} open={1} /> : <Missing what="the table" />}
          </Section>

          <Section title="Everything the client sent" id="s-client">
            <JsonTree value={client} open={1} />
          </Section>
        </div>

        <aside className="detail-side">
          <Section title="Triage" id="s-triage">
            <label htmlFor="d-status">Status</label>
            <select
              id="d-status"
              value={report.status}
              disabled={busy}
              onChange={(event) => {
                const status = STATUSES.find((s) => s === event.target.value);
                if (status) void change({ status }, `Status: ${STATUS_LABELS[status]}.`);
              }}
            >
              {STATUSES.map((s) => (
                <option key={s} value={s}>
                  {STATUS_LABELS[s]}
                </option>
              ))}
            </select>

            <h3>GitHub issue</h3>
            {report.issue_number !== null && report.issue_url !== null ? (
              <p>
                <a href={report.issue_url} target="_blank" rel="noopener noreferrer">
                  #{report.issue_number}
                </a>{" "}
                <button
                  type="button"
                  className="quiet"
                  disabled={busy}
                  onClick={() => void change({ issue: null }, "Unlinked.")}
                >
                  Unlink
                </button>
              </p>
            ) : (
              <p className="muted">Not linked.</p>
            )}
            <form className="inline" onSubmit={link}>
              <label htmlFor="d-issue" className="sr-only">
                Issue number or URL
              </label>
              <input
                id="d-issue"
                placeholder="#311 or issue URL"
                value={issueDraft}
                onChange={(event) => {
                  setIssueDraft(event.target.value);
                }}
              />
              <button type="submit" disabled={busy || issueDraft.trim() === ""}>
                Link
              </button>
            </form>
            <p>
              <a
                className="button"
                href={newIssueUrl(report, window.location.origin)}
                target="_blank"
                rel="noopener noreferrer"
              >
                Open a new issue on GitHub
              </a>
            </p>

            {notice !== null && (
              <output className="notice">
                {notice}
              </output>
            )}
            {error !== null && (
              <p className="error" role="alert">
                {error}
              </p>
            )}
          </Section>

          <Section title="Game record" id="s-record">
            {report.has_record ? (
              <>
                <p>
                  {formatBytes(report.record_bytes)}, {report.record_complete ? "complete" : "partial"}
                </p>
                <a className="button" href={api.recordUrl(report.id)} download={`${report.id}.jsonl.gz`}>
                  Download
                </a>
                <p className="muted small">
                  gzip, JSON Lines inside; replays with <code>baylee_gamehost::record::replay</code> on the
                  build its header names.
                </p>
              </>
            ) : (
              <p className="muted">No record is attached.</p>
            )}
          </Section>

          <Section title="System" id="s-system">
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
              <Missing what="system details" />
            )}
          </Section>

          <Section title="Settings" id="s-settings">
            {settings ? <JsonTree value={settings} open={1} /> : <Missing what="settings" />}
          </Section>

          <Section title="History" id="s-audit">
            {audit.length === 0 ? (
              <p className="muted">Nobody has changed it yet.</p>
            ) : (
              <ol className="audit">
                {audit.map((entry, i) => (
                  <li key={i}>
                    <span className="muted">{formatTime(entry.at)}</span> {entry.actor}:{" "}
                    {entry.action}
                    {entry.detail !== null ? ` ${entry.detail}` : entry.action === "issue" ? " unlinked" : ""}
                  </li>
                ))}
              </ol>
            )}
          </Section>

          <Section title="Delete" id="s-delete">
            {confirmDelete ? (
              <div className="confirm">
                <p>Delete this report and its record for good?</p>
                <button type="button" className="danger" disabled={busy} onClick={() => void remove()}>
                  Delete for good
                </button>{" "}
                <button
                  type="button"
                  className="quiet"
                  onClick={() => {
                    setConfirmDelete(false);
                  }}
                >
                  Keep it
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
                Delete report…
              </button>
            )}
          </Section>
        </aside>
      </div>
    </main>
  );
}

function BackLink() {
  return (
    <p>
      <a
        href="/"
        onClick={(event) => {
          event.preventDefault();
          navigate(listPath());
        }}
      >
        ← All reports
      </a>
    </p>
  );
}
