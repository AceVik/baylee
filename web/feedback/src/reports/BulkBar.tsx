// What the list does to the reports selected in it (`ReportList`): one
// request for all of them (`POST /ui/api/reports/bulk`), so either every
// one changes or none does.

import { useState } from "react";

import { PRIORITIES, STATUSES, type BulkAction, type BulkChange, type Summary } from "../api";
import { firstLine } from "../github";
import { formatCount, priorityLabel, statusLabel, t, type Lang } from "../i18n";
import { shortId } from "./pieces";

const LABEL = /^[a-z0-9][a-z0-9-]{0,31}$/;

/** The assign menu's "nobody": a value no admin's name can be. */
const NOBODY = "\u0000nobody";

export function BulkBar({
  lang,
  selected,
  me,
  admins,
  labels,
  inTrash,
  busy,
  onChange,
  onAction,
  onSeen,
  onCopyIds,
  onExport,
  onDeselect,
}: {
  lang: Lang;
  selected: readonly Summary[];
  me: string;
  admins: readonly string[];
  /** The labels in use, to offer. */
  labels: readonly string[];
  inTrash: boolean;
  busy: boolean;
  onChange: (change: BulkChange, ids?: readonly string[]) => void;
  onAction: (action: BulkAction) => void;
  onSeen: (seen: boolean) => void;
  onCopyIds: () => void;
  onExport: () => void;
  onDeselect: () => void;
}) {
  const [label, setLabel] = useState("");
  const [confirmPurge, setConfirmPurge] = useState(false);
  const people = [...new Set([me, ...admins])].filter((a) => a !== "");
  const theirLabels = [...new Set(selected.flatMap((r) => r.labels))].sort();
  const n = formatCount(lang, selected.length);
  const addLabel = () => {
    const value = label.trim().toLowerCase();
    if (!LABEL.test(value)) return;
    setLabel("");
    onChange({ labels_add: [value] });
  };
  return (
    <div className="bulk" role="toolbar" aria-label={t(lang, "rep.selected", { n: selected.length })}>
      <strong>{t(lang, "rep.selected", { n })}</strong>
      {inTrash ? (
        <>
          <button type="button" disabled={busy} onClick={() => { onAction("restore"); }}>
            {t(lang, "rep.restore")}
          </button>
          {confirmPurge ? (
            <span className="confirm-inline">
              {t(lang, "rep.purgeAsk", { n })}{" "}
              <button
                type="button"
                className="danger"
                disabled={busy}
                onClick={() => {
                  setConfirmPurge(false);
                  onAction("purge");
                }}
              >
                {t(lang, "rep.deleteYes")}
              </button>{" "}
              <button type="button" className="quiet" onClick={() => { setConfirmPurge(false); }}>
                {t(lang, "rep.deleteNo")}
              </button>
            </span>
          ) : (
            <button type="button" className="danger-quiet" disabled={busy} onClick={() => { setConfirmPurge(true); }}>
              {t(lang, "rep.purge")}
            </button>
          )}
        </>
      ) : (
        <>
          <label htmlFor="bulk-status" className="sr-only">
            {t(lang, "rep.setStatus")}
          </label>
          <select
            id="bulk-status"
            value=""
            disabled={busy}
            onChange={(event) => {
              const status = STATUSES.find((s) => s === event.target.value);
              if (status) onChange({ status });
            }}
          >
            <option value="">{t(lang, "rep.setStatus")}…</option>
            {STATUSES.map((s) => (
              <option key={s} value={s}>
                {statusLabel(lang, s)}
              </option>
            ))}
          </select>
          <label htmlFor="bulk-priority" className="sr-only">
            {t(lang, "rep.setPriority")}
          </label>
          <select
            id="bulk-priority"
            value=""
            disabled={busy}
            onChange={(event) => {
              const v = event.target.value;
              if (v === "none") onChange({ priority: null });
              else {
                const priority = PRIORITIES.find((p) => p === v);
                if (priority) onChange({ priority });
              }
            }}
          >
            <option value="">{t(lang, "rep.setPriority")}…</option>
            {PRIORITIES.map((p) => (
              <option key={p} value={p}>
                {priorityLabel(lang, p)}
              </option>
            ))}
            <option value="none">{t(lang, "rep.noPriority")}</option>
          </select>
          <label htmlFor="bulk-assign" className="sr-only">
            {t(lang, "rep.assign")}
          </label>
          <select
            id="bulk-assign"
            value=""
            disabled={busy}
            onChange={(event) => {
              const v = event.target.value;
              if (v === NOBODY) onChange({ assignee: null });
              else if (v !== "") onChange({ assignee: v });
            }}
          >
            <option value="">{t(lang, "rep.assign")}…</option>
            {people.map((name) => (
              <option key={name} value={name}>
                {name === me ? t(lang, "rep.me", { name }) : name}
              </option>
            ))}
            <option value={NOBODY}>{t(lang, "rep.nobody")}</option>
          </select>
          <span className="label-add">
            <label htmlFor="bulk-label" className="sr-only">
              {t(lang, "rep.addLabel")}
            </label>
            <input
              id="bulk-label"
              list="bulk-labels"
              size={10}
              placeholder={t(lang, "rep.addLabel")}
              value={label}
              disabled={busy}
              onChange={(event) => {
                setLabel(event.target.value);
              }}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  addLabel();
                }
              }}
            />
            <datalist id="bulk-labels">
              {labels.map((l) => (
                <option key={l} value={l}>
                  {l}
                </option>
              ))}
            </datalist>
            <button type="button" disabled={busy || !LABEL.test(label.trim().toLowerCase())} onClick={addLabel}>
              +
            </button>
          </span>
          {theirLabels.length > 0 && (
            <>
              <label htmlFor="bulk-unlabel" className="sr-only">
                {t(lang, "rep.removeLabel")}
              </label>
              <select
                id="bulk-unlabel"
                value=""
                disabled={busy}
                onChange={(event) => {
                  if (event.target.value !== "") onChange({ labels_remove: [event.target.value] });
                }}
              >
                <option value="">{t(lang, "rep.removeLabel")}…</option>
                {theirLabels.map((l) => (
                  <option key={l} value={l}>
                    {l}
                  </option>
                ))}
              </select>
            </>
          )}
          {selected.length > 1 && (
            <>
              <label htmlFor="bulk-merge" className="sr-only">
                {t(lang, "rep.mergeHint")}
              </label>
              <select
                id="bulk-merge"
                value=""
                title={t(lang, "rep.mergeHint")}
                disabled={busy}
                onChange={(event) => {
                  const original = event.target.value;
                  if (original === "") return;
                  onChange(
                    { duplicate_of: original },
                    selected.filter((r) => r.id !== original).map((r) => r.id),
                  );
                }}
              >
                <option value="">{t(lang, "rep.mergeInto")}</option>
                {selected
                  .filter((r) => r.duplicate_of === null)
                  .map((r) => (
                    <option key={r.id} value={r.id}>
                      {shortId(r.id)} · {firstLine(r.text, 40) || t(lang, "rep.noText")}
                    </option>
                  ))}
              </select>
            </>
          )}
          <button type="button" disabled={busy} onClick={() => { onAction("trash"); }}>
            {t(lang, "rep.toTrash")}
          </button>
        </>
      )}
      <button type="button" onClick={() => { onSeen(true); }}>
        {t(lang, "rep.markRead")}
      </button>
      <button type="button" onClick={() => { onSeen(false); }}>
        {t(lang, "rep.markUnread")}
      </button>
      <button type="button" onClick={onCopyIds}>
        {t(lang, "rep.copyIds")}
      </button>
      <button type="button" onClick={onExport}>
        {t(lang, "rep.exportSelected")}
      </button>
      <button type="button" className="quiet" onClick={onDeselect}>
        {t(lang, "rep.deselect")}
      </button>
    </div>
  );
}
