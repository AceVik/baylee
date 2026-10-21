// The triage controls of one report (`ReportView`): its priority, who
// looks after it, its labels, the report it duplicates, its internal
// notes, the reports it belongs with, the trash, a screenshot at full
// size, and its history in words.

import { useEffect, useState, type FormEvent, type ReactNode } from "react";

import {
  api,
  isReportId,
  TRASH_DAYS,
  type AuditEntry,
  type Change,
  type Filter,
  type Note,
  type Priority,
  type Report,
} from "../api";
import { formatCount, priorityLabel, statusLabel, t, type Lang } from "../i18n";
import { navigate } from "../router";
import { shortId, Time, TimeBoth } from "./pieces";

const LABEL = /^[a-z0-9][a-z0-9-]{0,31}$/;

/** The priorities from the least urgent up, beside "none". */
const LOW_FIRST: readonly Priority[] = ["low", "normal", "high", "critical"];
const UUID_IN = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i;

/** A change and what to say once it is made. */
export type Apply = (change: Change, done: string) => void;

/** An in-page link to the list under `filter`'s query. */
function ListLink({ query, children }: { query: string; children: ReactNode }) {
  return (
    <a
      href={`/?${query}`}
      onClick={(event) => {
        if (event.metaKey || event.ctrlKey || event.shiftKey) return;
        event.preventDefault();
        navigate(`/?${query}`);
      }}
    >
      {children}
    </a>
  );
}

/** A link to another report's page, by its short id. */
export function ReportLink({ id }: { id: string }) {
  return (
    <a
      className="mono"
      href={`/r/${id}`}
      onClick={(event) => {
        if (event.metaKey || event.ctrlKey || event.shiftKey) return;
        event.preventDefault();
        navigate(`/r/${id}`);
      }}
    >
      {shortId(id)}
    </a>
  );
}

export function PriorityPicker({ lang, report, busy, apply }: { lang: Lang; report: Report; busy: boolean; apply: Apply }) {
  return (
    <fieldset className="quick-status">
      <legend>{t(lang, "rep.priority")}</legend>
      <button
        type="button"
        className={`chip-button${report.priority === null ? " on" : ""}`}
        aria-pressed={report.priority === null}
        disabled={busy || report.priority === null}
        onClick={() => {
          apply({ priority: null }, t(lang, "rep.prioritySet", { priority: t(lang, "rep.noPriority") }));
        }}
      >
        {t(lang, "rep.noPriority")}
      </button>
      {LOW_FIRST.map((p) => (
        <button
          key={p}
          type="button"
          className={`chip-button prio-${p}${report.priority === p ? " on" : ""}`}
          aria-pressed={report.priority === p}
          disabled={busy || report.priority === p}
          onClick={() => {
            apply({ priority: p }, t(lang, "rep.prioritySet", { priority: priorityLabel(lang, p) }));
          }}
        >
          {priorityLabel(lang, p)}
        </button>
      ))}
    </fieldset>
  );
}

export function AssigneePicker({
  lang,
  report,
  me,
  admins,
  busy,
  apply,
}: {
  lang: Lang;
  report: Report;
  me: string;
  admins: readonly string[];
  busy: boolean;
  apply: Apply;
}) {
  const people = [...new Set([me, ...admins, ...(report.assignee === null ? [] : [report.assignee])])].filter(
    (a) => a !== "",
  );
  return (
    <div className="assignee">
      <label htmlFor="d-assignee">{t(lang, "rep.assignee")}</label>
      <span className="inline">
        <select
          id="d-assignee"
          value={report.assignee ?? ""}
          disabled={busy}
          onChange={(event) => {
            const name = event.target.value;
            if (name === "") apply({ assignee: null }, t(lang, "rep.unassigned"));
            else apply({ assignee: name }, t(lang, "rep.assigned", { name }));
          }}
        >
          <option value="">{t(lang, "rep.nobody")}</option>
          {people.map((name) => (
            <option key={name} value={name}>
              {name === me ? t(lang, "rep.me", { name }) : name}
            </option>
          ))}
        </select>
        {me !== "" && report.assignee !== me && (
          <button
            type="button"
            className="quiet"
            disabled={busy}
            onClick={() => {
              apply({ assignee: me }, t(lang, "rep.assigned", { name: me }));
            }}
          >
            {t(lang, "rep.assignMe")}
          </button>
        )}
      </span>
    </div>
  );
}

export function LabelEditor({ lang, report, busy, apply }: { lang: Lang; report: Report; busy: boolean; apply: Apply }) {
  const [draft, setDraft] = useState("");
  const [known, setKnown] = useState<string[] | null>(null);
  const [bad, setBad] = useState(false);
  const suggest = () => {
    if (known !== null) return;
    setKnown([]);
    api
      .facets({})
      .then((f) => {
        setKnown(f.labels.flatMap((c) => (c.value === null ? [] : [c.value])));
      })
      .catch(() => {
        // Labels are typed without suggestions.
      });
  };
  const add = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const label = draft.trim().toLowerCase();
    if (!LABEL.test(label)) {
      setBad(true);
      return;
    }
    setBad(false);
    setDraft("");
    if (!report.labels.includes(label)) apply({ labels: [...report.labels, label] }, t(lang, "rep.labelsSet"));
  };
  return (
    <div className="label-editor">
      <h3>{t(lang, "rep.labels")}</h3>
      {report.labels.length > 0 && (
        <ul className="labels">
          {report.labels.map((label) => (
            <li key={label} className="label-chip">
              <ListLink query={`label=${encodeURIComponent(label)}`}>{label}</ListLink>
              <button
                type="button"
                className="quiet"
                disabled={busy}
                aria-label={t(lang, "rep.removeLabelN", { label })}
                onClick={() => {
                  apply({ labels: report.labels.filter((l) => l !== label) }, t(lang, "rep.labelsSet"));
                }}
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
      <form className="inline" onSubmit={add}>
        <label htmlFor="d-label" className="sr-only">
          {t(lang, "rep.labelLabel")}
        </label>
        <input
          id="d-label"
          list="d-labels"
          size={14}
          placeholder={t(lang, "rep.labelHint")}
          value={draft}
          aria-invalid={bad || undefined}
          onFocus={suggest}
          onChange={(event) => {
            setDraft(event.target.value);
            setBad(false);
          }}
        />
        <datalist id="d-labels">
          {(known ?? [])
            .filter((l) => !report.labels.includes(l))
            .map((l) => (
              <option key={l} value={l}>
                {l}
              </option>
            ))}
        </datalist>
        <button type="submit" disabled={busy || draft.trim() === "" || report.labels.length >= 16}>
          {t(lang, "rep.addLabel")}
        </button>
      </form>
      {bad && (
        <p className="error small" role="alert">
          {t(lang, "rep.badLabel")}
        </p>
      )}
    </div>
  );
}

export function DuplicateEditor({ lang, report, busy, apply }: { lang: Lang; report: Report; busy: boolean; apply: Apply }) {
  const [draft, setDraft] = useState("");
  const [bad, setBad] = useState(false);
  if (report.duplicate_of !== null) {
    return (
      <div className="duplicate">
        <h3>{t(lang, "rep.duplicateOf")}</h3>
        <p>
          <ReportLink id={report.duplicate_of} />{" "}
          <button
            type="button"
            className="quiet"
            disabled={busy}
            onClick={() => {
              apply({ duplicate_of: null }, t(lang, "rep.duplicateCleared"));
            }}
          >
            {t(lang, "rep.notDuplicate")}
          </button>
        </p>
      </div>
    );
  }
  const mark = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const found = UUID_IN.exec(draft)?.[0]?.toLowerCase();
    if (found === undefined || !isReportId(found) || found === report.id) {
      setBad(true);
      return;
    }
    setBad(false);
    setDraft("");
    apply({ duplicate_of: found }, t(lang, "rep.duplicateSet"));
  };
  return (
    <div className="duplicate">
      <h3>{t(lang, "rep.markDuplicate")}</h3>
      <form className="inline" onSubmit={mark}>
        <label htmlFor="d-duplicate" className="sr-only">
          {t(lang, "rep.duplicateLabel")}
        </label>
        <input
          id="d-duplicate"
          spellCheck={false}
          placeholder={t(lang, "rep.duplicateLabel")}
          value={draft}
          aria-invalid={bad || undefined}
          onChange={(event) => {
            setDraft(event.target.value);
            setBad(false);
          }}
        />
        <button type="submit" disabled={busy || draft.trim() === ""}>
          {t(lang, "rep.markDuplicate")}
        </button>
      </form>
      {bad && (
        <p className="error small" role="alert">
          {t(lang, "rep.badDuplicate")}
        </p>
      )}
    </div>
  );
}

export function Notes({
  lang,
  report,
  me,
  onNotes,
  onError,
}: {
  lang: Lang;
  report: Report;
  me: string;
  onNotes: (notes: Note[]) => void;
  onError: (e: unknown) => void;
}) {
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const notes = report.note_list ?? [];
  const add = () => {
    const text = draft.trim();
    if (text === "" || busy) return;
    setBusy(true);
    api
      .addNote(report.id, text)
      .then((all) => {
        setDraft("");
        onNotes(all);
      })
      .catch(onError)
      .finally(() => {
        setBusy(false);
      });
  };
  return (
    <>
      <p className="muted small">{t(lang, "rep.notesHint")}</p>
      {notes.length === 0 ? (
        <p className="muted">{t(lang, "rep.noNotes")}</p>
      ) : (
        <ol className="notes">
          {notes.map((note) => (
            <li key={note.id}>
              <div className="note-head">
                <strong>{note.author}</strong> <Time lang={lang} iso={note.at} />
                {note.author === me && (
                  <button
                    type="button"
                    className="quiet small"
                    aria-label={t(lang, "rep.removeNote")}
                    title={t(lang, "rep.removeNote")}
                    onClick={() => {
                      api.removeNote(report.id, note.id).then(onNotes).catch(onError);
                    }}
                  >
                    ×
                  </button>
                )}
              </div>
              <p className="note-text">{note.text}</p>
            </li>
          ))}
        </ol>
      )}
      <form
        className="note-form"
        onSubmit={(event) => {
          event.preventDefault();
          add();
        }}
      >
        <label htmlFor="d-note">{t(lang, "rep.noteLabel")}</label>
        <textarea
          id="d-note"
          rows={3}
          maxLength={4000}
          value={draft}
          onChange={(event) => {
            setDraft(event.target.value);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
              event.preventDefault();
              add();
            }
          }}
        />
        <button type="submit" disabled={busy || draft.trim() === ""}>
          {t(lang, "rep.addNote")}
        </button>
      </form>
    </>
  );
}

/**
 * How many reports `filter` matches besides the one shown (`counted`: it
 * is among them); `null` until known.
 */
function useOthers(filter: Filter | null, counted: boolean): number | null {
  const [count, setCount] = useState<number | null>(null);
  const key = filter === null ? "" : JSON.stringify(filter);
  useEffect(() => {
    if (key === "") return;
    let live = true;
    const wanted = JSON.parse(key) as Filter;
    api
      .reports({ ...wanted, limit: 1 })
      .then((l) => {
        if (live) setCount(Math.max(0, l.total - (counted ? 1 : 0)));
      })
      .catch(() => {
        // The link works without its count.
      });
    return () => {
      live = false;
    };
  }, [key, counted]);
  return count;
}

export function Related({ lang, report }: { lang: Lang; report: Report }) {
  // A report in the trash is in no list but the trash's.
  const listed = report.deleted_at === null;
  const game = useOthers(report.game_id === null ? null : { game_id: report.game_id }, listed);
  const crash = useOthers(report.signature === null ? null : { signature: report.signature }, listed);
  const duplicates = useOthers({ duplicate_of: report.id }, false);
  const n = (v: number | null) => (v === null ? "" : ` (${formatCount(lang, v)})`);
  const rows: ReactNode[] = [];
  if (report.duplicate_of !== null) {
    rows.push(
      <li key="original">
        {t(lang, "rep.theOriginal")}: <ReportLink id={report.duplicate_of} />
      </li>,
    );
  }
  if (duplicates !== null && duplicates > 0) {
    rows.push(
      <li key="dups">
        <ListLink query={`duplicate_of=${report.id}`}>
          {t(lang, "rep.itsDuplicates")}
          {n(duplicates)}
        </ListLink>
      </li>,
    );
  }
  if (report.signature !== null && crash !== 0) {
    rows.push(
      <li key="crash">
        <ListLink query={`signature=${report.signature}`}>
          {t(lang, "rep.sameCrash")}
          {n(crash)}
        </ListLink>
      </li>,
    );
  }
  if (report.game_id !== null && game !== 0) {
    rows.push(
      <li key="game">
        <ListLink query={`game_id=${encodeURIComponent(report.game_id)}`}>
          {t(lang, "rep.sameGame")}
          {n(game)}
        </ListLink>
      </li>,
    );
  }
  return rows.length === 0 ? <p className="muted">{t(lang, "rep.nothingRelated")}</p> : <ul className="related">{rows}</ul>;
}

/** Above a report in the trash: since when, and the two ways out of it. */
export function TrashBanner({
  lang,
  report,
  busy,
  onRestore,
  onPurge,
}: {
  lang: Lang;
  report: Report;
  busy: boolean;
  onRestore: () => void;
  onPurge: () => void;
}) {
  const [asking, setAsking] = useState(false);
  if (report.deleted_at === null) return null;
  return (
    <div className="trash-banner">
      <p>
        {t(lang, "rep.inTrashSince")} <TimeBoth lang={lang} iso={report.deleted_at} />.{" "}
        {t(lang, "rep.trashDays", { days: TRASH_DAYS })}
      </p>
      <p>
        <button type="button" className="primary" disabled={busy} onClick={onRestore}>
          {t(lang, "rep.restore")}
        </button>{" "}
        {asking ? (
          <>
            {t(lang, "rep.deleteAsk")}{" "}
            <button type="button" className="danger" disabled={busy} onClick={onPurge}>
              {t(lang, "rep.deleteYes")}
            </button>{" "}
            <button
              type="button"
              className="quiet"
              onClick={() => {
                setAsking(false);
              }}
            >
              {t(lang, "rep.deleteNo")}
            </button>
          </>
        ) : (
          <button
            type="button"
            className="danger-quiet"
            disabled={busy}
            onClick={() => {
              setAsking(true);
            }}
          >
            {t(lang, "rep.purge")}
          </button>
        )}
      </p>
    </div>
  );
}

/** The screenshot at full size over the page; `Escape` or a click closes it. */
export function Lightbox({ lang, src, onClose }: { lang: Lang; src: string; onClose: () => void }) {
  useEffect(() => {
    // Caught before the page's own `Escape` (back to the list, or the
    // drawer shut), which waits on the window's bubbling side.
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
    };
  }, [onClose]);
  return (
    <dialog className="lightbox" open aria-label={t(lang, "rep.screenshot")}>
      <button type="button" className="lightbox-close quiet" aria-label={t(lang, "rep.close")} onClick={onClose}>
        ×
      </button>
      <button type="button" className="lightbox-image" aria-label={t(lang, "rep.close")} onClick={onClose}>
        <img src={src} alt={t(lang, "rep.screenshotAlt")} />
      </button>
    </dialog>
  );
}

/** An audit line's action in words. */
export function auditText(lang: Lang, entry: AuditEntry): string {
  const d = entry.detail;
  switch (entry.action) {
    case "status":
      return `${t(lang, "audit.status")} ${statusLabel(lang, d ?? "")}`;
    case "issue":
      return d ? `${t(lang, "audit.issue")} ${d}` : t(lang, "audit.unlinked");
    case "assign":
      return d ? `${t(lang, "audit.assign")} ${d}` : t(lang, "audit.unassign");
    case "priority":
      return d ? `${t(lang, "audit.priority")} ${priorityLabel(lang, d)}` : t(lang, "audit.unprioritised");
    case "labels":
      return d ? `${t(lang, "audit.labels")} ${d.split(",").join(", ")}` : t(lang, "audit.nolabels");
    case "duplicate":
      return d ? `${t(lang, "audit.duplicate")} ${shortId(d)}` : t(lang, "audit.original");
    case "trash":
      return t(lang, "audit.trash");
    case "restore":
      return t(lang, "audit.restore");
    case "delete":
      return t(lang, "audit.delete");
    default:
      return d === null ? entry.action : `${entry.action} ${d}`;
  }
}
