// Who may seat which hosted model (docs/feedback.md §"Hosted models",
// docs/protocol.md §"Who may seat which model"): per profile id a policy
// (every registered account, only the accounts on its list, or nobody) and
// the list itself, edited in a dialog with a type-ahead over registered
// accounts, chips for what is about to change, and a review before saving.
// A change stops no game and no chair already ordered.

// The type-ahead's popup is a WAI-ARIA 1.2 combobox listbox: a native
// <select> or <datalist> cannot show a handle beside a username or mark an
// account already on the list, so its roles are written out.
/* oxlint-disable jsx-a11y/prefer-tag-over-role */

import { useCallback, useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from "react";

import {
  api,
  LLM_POLICIES,
  LOOKUP_MIN,
  SignedOut,
  type AccountCandidate,
  type LlmAccess,
  type LlmAccessDetail,
  type LlmAllowed,
  type LlmPolicy,
} from "../api";
import { formatAgo, formatCount, formatWhen, t, type Lang } from "../i18n";
import { Badge, describe, Panel, type Tone } from "./shared";
import { Modal } from "./Models";

const POLICY_TONE: Record<LlmPolicy, Tone> = { registered: "ok", listed: "info", nobody: "muted" };

/** How long the type-ahead waits after the last key before it asks. */
export const LOOKUP_DEBOUNCE_MS = 250;

/** One account as the editor shows it: `Alice#af03`, and `@alice` when it has a username. */
export interface Person {
  id: string;
  handle: string;
  username: string | null;
}

export type ListSort = "name" | "newest" | "oldest";

const byName = (a: LlmAllowed, b: LlmAllowed) =>
  (a.username ?? a.display_name).localeCompare(b.username ?? b.display_name) || a.handle.localeCompare(b.handle);

/** The list filtered by `query` (handle or username, any case) and sorted. */
export function shownList(list: LlmAllowed[], query: string, sort: ListSort): LlmAllowed[] {
  const q = query.trim().toLowerCase();
  const kept = q === "" ? list : list.filter((a) => a.handle.toLowerCase().includes(q) || (a.username ?? "").toLowerCase().includes(q));
  return [...kept].sort((a, b) => {
    if (sort === "newest") return b.added_at.localeCompare(a.added_at) || byName(a, b);
    if (sort === "oldest") return a.added_at.localeCompare(b.added_at) || byName(a, b);
    return byName(a, b);
  });
}

/** What saving would change: the policy (when it moves), the accounts to add and to take off. */
export interface AccessPlan {
  policy: LlmPolicy | null;
  makeDefault: boolean | null;
  add: Person[];
  remove: Person[];
}

export function accessPlan(
  detail: LlmAccessDetail,
  policy: LlmPolicy,
  isDefault: boolean,
  add: Person[],
  remove: Person[],
): AccessPlan {
  const listed = new Set(detail.accounts.map((a) => a.id));
  const policyMoves = !detail.stored || policy !== detail.policy;
  const defaultMoves = isDefault !== detail.default_for_registered;
  return {
    policy: policyMoves || defaultMoves ? policy : null,
    makeDefault: defaultMoves ? isDefault : null,
    add: add.filter((p) => !listed.has(p.id)),
    remove: remove.filter((p) => listed.has(p.id)),
  };
}

export function planIsEmpty(plan: AccessPlan): boolean {
  return plan.policy === null && plan.add.length === 0 && plan.remove.length === 0;
}

/** The type-ahead: a combobox over registered accounts, debounced, with keyboard navigation. */
function AccountPicker({
  lang,
  taken,
  onPick,
}: {
  lang: Lang;
  /** Ids already on the list or about to be added: shown, not pickable. */
  taken: Set<string>;
  onPick: (person: Person) => void;
}) {
  const base = useId();
  const listId = `${base}-list`;
  const [text, setText] = useState("");
  /** The latest answer, with the search it answers. */
  const [answer, setAnswer] = useState<{ q: string; accounts: AccountCandidate[] } | null>(null);
  const [active, setActive] = useState(0);
  const [open, setOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const asked = useRef(0);

  const q = text.trim();
  useEffect(() => {
    if (q.length < LOOKUP_MIN) return;
    const ticket = ++asked.current;
    const timer = setTimeout(() => {
      api.admin
        .lookup(q)
        .then((accounts) => {
          if (ticket !== asked.current) return;
          setAnswer({ q, accounts });
          setActive(0);
          setError(null);
        })
        .catch((e: unknown) => {
          if (ticket !== asked.current || e instanceof SignedOut) return;
          setError(describe(lang, e));
        });
    }, LOOKUP_DEBOUNCE_MS);
    return () => {
      clearTimeout(timer);
    };
  }, [q, lang]);

  // Only an answer to what the field says now counts.
  const found = q.length >= LOOKUP_MIN && answer !== null && answer.q === q ? answer.accounts : null;
  const options = found ?? [];
  const pick = (c: AccountCandidate | undefined) => {
    if (c === undefined || taken.has(c.id)) return;
    onPick({ id: c.id, handle: c.handle, username: c.username });
    setText("");
    setAnswer(null);
    setOpen(false);
  };
  const onKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown" && options.length > 0) {
      event.preventDefault();
      setOpen(true);
      setActive((i) => (i + 1) % options.length);
    } else if (event.key === "ArrowUp" && options.length > 0) {
      event.preventDefault();
      setOpen(true);
      setActive((i) => (i - 1 + options.length) % options.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (open) pick(options[active]);
    } else if (event.key === "Escape" && open) {
      event.preventDefault();
      event.stopPropagation();
      setOpen(false);
    }
  };
  const showList = open && q.length >= LOOKUP_MIN;
  const hint =
    q.length < LOOKUP_MIN
      ? t(lang, "access.typeMore", { n: LOOKUP_MIN })
      : found === null
        ? t(lang, "access.searching")
        : found.length === 0
          ? t(lang, "access.noMatch")
          : t(lang, "access.matches", { n: formatCount(lang, found.length) });

  return (
    <div className="access-picker">
      <label htmlFor={`${base}-input`}>{t(lang, "access.find")}</label>
      <input
        id={`${base}-input`}
        type="search"
        role="combobox"
        aria-expanded={showList && options.length > 0}
        aria-controls={listId}
        aria-autocomplete="list"
        aria-activedescendant={showList && options[active] !== undefined ? `${base}-opt-${active}` : undefined}
        aria-describedby={`${base}-hint`}
        autoComplete="off"
        spellCheck={false}
        placeholder={t(lang, "access.findPlaceholder")}
        value={text}
        onChange={(e) => {
          setText(e.target.value);
          setOpen(true);
        }}
        onFocus={() => {
          setOpen(true);
        }}
        onBlur={() => {
          setOpen(false);
        }}
        onKeyDown={onKey}
      />
      <p id={`${base}-hint`} className="muted small" aria-live="polite">
        {error ?? hint}
      </p>
      {showList && options.length > 0 && (
        <div id={listId} role="listbox" className="access-options" aria-label={t(lang, "access.find")}>
          {options.map((c, i) => {
            const isTaken = taken.has(c.id);
            return (
              <div
                key={c.id}
                id={`${base}-opt-${i}`}
                tabIndex={-1}
                role="option"
                aria-selected={i === active}
                aria-disabled={isTaken}
                className={`access-option${i === active ? " active" : ""}${isTaken ? " taken" : ""}`}
                // Before the input's blur closes the list.
                onMouseDown={(e) => {
                  e.preventDefault();
                  pick(c);
                }}
                onMouseEnter={() => {
                  setActive(i);
                }}
              >
                <strong>{c.handle}</strong>
                {c.username !== null && <span className="muted mono small"> @{c.username}</span>}
                {isTaken && <span className="muted small"> · {t(lang, "access.already")}</span>}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function Chip({ person, label, onRemove }: { person: Person; label: string; onRemove: () => void }) {
  return (
    <li className="access-chip">
      <span>{person.handle}</span>
      {person.username !== null && <span className="muted mono small">@{person.username}</span>}
      <button type="button" className="access-chip-x" aria-label={label} onClick={onRemove}>
        <span aria-hidden="true">×</span>
      </button>
    </li>
  );
}

/** The dialog for one profile: its policy, its list, and a review before anything is sent. */
function AccessEditor({
  lang,
  access,
  onDone,
  onClose,
}: {
  lang: Lang;
  access: LlmAccess;
  onDone: (text: string) => void;
  onClose: () => void;
}) {
  const [detail, setDetail] = useState<LlmAccessDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [policy, setPolicy] = useState<LlmPolicy>(access.policy);
  const [isDefault, setIsDefault] = useState(access.default_for_registered);
  const [adding, setAdding] = useState<Person[]>([]);
  const [removing, setRemoving] = useState<Person[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [filter, setFilter] = useState("");
  const [sort, setSort] = useState<ListSort>("name");
  const [reviewing, setReviewing] = useState(false);
  const [busy, setBusy] = useState(false);
  const name = access.label ?? access.id;

  useEffect(() => {
    api.admin.llm
      .accessOf(access.id)
      .then((d) => {
        setDetail(d);
        setPolicy(d.policy);
        setIsDefault(d.default_for_registered);
      })
      .catch((e: unknown) => {
        if (!(e instanceof SignedOut)) setError(describe(lang, e));
      });
  }, [access.id, lang]);

  const removingIds = useMemo(() => new Set(removing.map((p) => p.id)), [removing]);
  const taken = useMemo(
    () => new Set([...(detail?.accounts ?? []).map((a) => a.id), ...adding.map((p) => p.id)]),
    [detail, adding],
  );
  const shown = useMemo(() => shownList(detail?.accounts ?? [], filter, sort), [detail, filter, sort]);
  const plan = detail === null ? null : accessPlan(detail, policy, isDefault, adding, removing);

  const toggleRemove = (a: LlmAllowed) => {
    setRemoving((list) =>
      list.some((p) => p.id === a.id) ? list.filter((p) => p.id !== a.id) : [...list, { id: a.id, handle: a.handle, username: a.username }],
    );
  };
  const removeSelected = () => {
    const chosen = (detail?.accounts ?? []).filter((a) => selected.has(a.id) && !removingIds.has(a.id));
    setRemoving((list) => [...list, ...chosen.map((a) => ({ id: a.id, handle: a.handle, username: a.username }))]);
    setSelected(new Set());
  };
  const allShownSelected = shown.length > 0 && shown.every((a) => selected.has(a.id));

  const save = async () => {
    if (plan === null) return;
    setBusy(true);
    setError(null);
    try {
      if (plan.policy !== null) {
        await api.admin.llm.setPolicy(access.id, plan.policy, plan.makeDefault ?? undefined);
      }
      if (plan.add.length > 0) await api.admin.llm.allow(access.id, plan.add.map((p) => p.id));
      if (plan.remove.length > 0) await api.admin.llm.disallow(access.id, plan.remove.map((p) => p.id));
      onDone(t(lang, "access.saved", { name }));
      onClose();
    } catch (e: unknown) {
      if (!(e instanceof SignedOut)) setError(describe(lang, e));
      setReviewing(false);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal labelledBy="access-title" className="access-editor" onClose={onClose}>
      <h3 id="access-title">{t(lang, "access.editTitle", { name })}</h3>
      <p className="muted small">
        <span className="mono">{access.id}</span>
        {access.vendor !== null && ` · ${access.vendor}`} ·{" "}
        {access.connected ? t(lang, "access.reported") : t(lang, "access.notReported")}
      </p>
      {detail === null ? (
        error === null ? (
          <p className="muted">{t(lang, "admin.loading")}</p>
        ) : null
      ) : reviewing && plan !== null ? (
        <section className="access-review" aria-labelledby="access-review-title">
          <h4 id="access-review-title">{t(lang, "access.review")}</h4>
          <ul>
            {plan.policy !== null && (
              <li>
                {t(lang, "access.reviewPolicy", { policy: t(lang, `access.policy.${plan.policy}`) })}
                {plan.makeDefault === true && ` ${t(lang, "access.reviewDefault")}`}
                {plan.makeDefault === false && ` ${t(lang, "access.reviewNotDefault")}`}
              </li>
            )}
            {plan.add.length > 0 && (
              <li>
                {t(lang, "access.reviewAdd", { n: plan.add.length })}{" "}
                <span className="muted">{plan.add.map((p) => p.handle).join(", ")}</span>
              </li>
            )}
            {plan.remove.length > 0 && (
              <li>
                {t(lang, "access.reviewRemove", { n: plan.remove.length })}{" "}
                <span className="muted">{plan.remove.map((p) => p.handle).join(", ")}</span>
              </li>
            )}
          </ul>
          <p className="muted small">{t(lang, "access.reviewNote")}</p>
          <div className="row-actions">
            <button type="button" className="primary" disabled={busy} onClick={() => void save()}>
              {busy ? t(lang, "llm.saving") : t(lang, "access.confirm")}
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => {
                setReviewing(false);
              }}
            >
              {t(lang, "access.back")}
            </button>
          </div>
        </section>
      ) : (
        <div className="access-form">
          <fieldset className="access-policies">
            <legend>{t(lang, "access.policy")}</legend>
            {LLM_POLICIES.map((p) => (
              <label key={p} className={`access-policy${policy === p ? " on" : ""}`}>
                <input
                  type="radio"
                  name="access-policy"
                  value={p}
                  checked={policy === p}
                  onChange={() => {
                    setPolicy(p);
                    if (p !== "registered") setIsDefault(false);
                  }}
                />
                <span>
                  <strong>{t(lang, `access.policy.${p}`)}</strong>
                  <span className="muted small">{t(lang, `access.policyHint.${p}`)}</span>
                </span>
              </label>
            ))}
            <label className="access-default">
              <input
                type="checkbox"
                checked={isDefault}
                onChange={(e) => {
                  setIsDefault(e.target.checked);
                  if (e.target.checked) setPolicy("registered");
                }}
              />{" "}
              {t(lang, "access.default")}
            </label>
            <p className="muted small">{t(lang, "access.defaultHint")}</p>
          </fieldset>

          <section className={`access-list${policy === "listed" ? "" : " dim"}`} aria-labelledby="access-list-title">
            <h4 id="access-list-title">
              {t(lang, "access.list")}{" "}
              <span className="muted small">({formatCount(lang, detail.accounts.length)})</span>
            </h4>
            {policy !== "listed" && <p className="muted small">{t(lang, "access.listIdle")}</p>}
            <AccountPicker
              lang={lang}
              taken={taken}
              onPick={(person) => {
                setAdding((list) => [...list, person]);
                setRemoving((list) => list.filter((p) => p.id !== person.id));
              }}
            />
            {adding.length > 0 && (
              <div className="access-pending">
                <p className="small">{t(lang, "access.toAdd", { n: formatCount(lang, adding.length) })}</p>
                <ul className="access-chips">
                  {adding.map((p) => (
                    <Chip
                      key={p.id}
                      person={p}
                      label={t(lang, "access.dropAdd", { who: p.handle })}
                      onRemove={() => {
                        setAdding((list) => list.filter((q) => q.id !== p.id));
                      }}
                    />
                  ))}
                </ul>
              </div>
            )}
            {removing.length > 0 && (
              <div className="access-pending removing">
                <p className="small">{t(lang, "access.toRemove", { n: formatCount(lang, removing.length) })}</p>
                <ul className="access-chips">
                  {removing.map((p) => (
                    <Chip
                      key={p.id}
                      person={p}
                      label={t(lang, "access.keep", { who: p.handle })}
                      onRemove={() => {
                        setRemoving((list) => list.filter((q) => q.id !== p.id));
                      }}
                    />
                  ))}
                </ul>
              </div>
            )}

            {detail.accounts.length === 0 ? (
              <p className="empty">{t(lang, "access.empty")}</p>
            ) : (
              <>
                <div className="access-tools">
                  <input
                    type="search"
                    aria-label={t(lang, "access.filter")}
                    placeholder={t(lang, "access.filter")}
                    value={filter}
                    onChange={(e) => {
                      setFilter(e.target.value);
                    }}
                  />
                  <select
                    aria-label={t(lang, "access.sort")}
                    value={sort}
                    onChange={(e) => {
                      setSort(e.target.value as ListSort);
                    }}
                  >
                    <option value="name">{t(lang, "access.sortName")}</option>
                    <option value="newest">{t(lang, "access.sortNewest")}</option>
                    <option value="oldest">{t(lang, "access.sortOldest")}</option>
                  </select>
                  <label className="small">
                    <input
                      type="checkbox"
                      checked={allShownSelected}
                      onChange={() => {
                        setSelected(allShownSelected ? new Set() : new Set(shown.map((a) => a.id)));
                      }}
                    />{" "}
                    {t(lang, "access.selectShown")}
                  </label>
                  <button type="button" className="danger-quiet" disabled={selected.size === 0} onClick={removeSelected}>
                    {t(lang, "access.removeSelected", { n: formatCount(lang, selected.size) })}
                  </button>
                </div>
                {shown.length === 0 ? (
                  <p className="empty">{t(lang, "access.noneShown")}</p>
                ) : (
                  <ul className="access-rows">
                    {shown.map((a) => {
                      const going = removingIds.has(a.id);
                      return (
                        <li key={a.id} className={`access-row${going ? " going" : ""}`} data-testid="access-row">
                          <input
                            type="checkbox"
                            aria-label={t(lang, "access.select", { who: a.handle })}
                            checked={selected.has(a.id)}
                            onChange={() => {
                              setSelected((s) => {
                                const next = new Set(s);
                                if (next.has(a.id)) next.delete(a.id);
                                else next.add(a.id);
                                return next;
                              });
                            }}
                          />
                          <span className="access-who">
                            <strong>{a.handle}</strong>
                            {a.username !== null && <span className="muted mono small"> @{a.username}</span>}
                          </span>
                          <span className="muted small" title={formatWhen(lang, a.added_at)}>
                            {t(lang, "access.addedBy", { ago: formatAgo(lang, a.added_at), by: a.added_by })}
                          </span>
                          <button
                            type="button"
                            className={going ? "quiet" : "danger-quiet"}
                            aria-label={`${going ? t(lang, "access.undo") : t(lang, "access.remove")} ${a.handle}`}
                            onClick={() => {
                              toggleRemove(a);
                            }}
                          >
                            {going ? t(lang, "access.undo") : t(lang, "access.remove")}
                          </button>
                        </li>
                      );
                    })}
                  </ul>
                )}
              </>
            )}
          </section>
          <div className="row-actions">
            <button
              type="button"
              className="primary"
              disabled={plan === null || planIsEmpty(plan)}
              onClick={() => {
                setReviewing(true);
              }}
            >
              {t(lang, "access.saveEllipsis")}
            </button>
            <button type="button" onClick={onClose}>
              {t(lang, "llm.cancel")}
            </button>
          </div>
        </div>
      )}
      {error !== null && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </Modal>
  );
}

/** The panel on the Models page: every profile a seat agent reports or a policy names. */
export function AccessPanel({
  lang,
  access,
  onChanged,
}: {
  lang: Lang;
  access: LlmAccess[] | null;
  onChanged: (text: string) => void;
}) {
  const [editing, setEditing] = useState<LlmAccess | null>(null);
  const close = useCallback(() => {
    setEditing(null);
  }, []);
  return (
    <Panel id="access-panel-title" title={t(lang, "access.title")}>
      <p className="muted small">{t(lang, "access.hint")}</p>
      {access === null ? (
        <p className="muted">{t(lang, "admin.loading")}</p>
      ) : access.length === 0 ? (
        <p className="empty">{t(lang, "access.noProfiles")}</p>
      ) : (
        <ul className="access-profiles">
          {access.map((a) => (
            <li key={a.id} className="access-profile card" data-testid="access-profile">
              <div>
                <strong>{a.label ?? a.id}</strong>{" "}
                <span className="muted small mono">{a.id}</span>
                <div className="llm-badges">
                  <Badge tone={a.stored ? POLICY_TONE[a.policy] : "warn"} dot>
                    {a.stored ? t(lang, `access.policy.${a.policy}`) : t(lang, "access.unset")}
                  </Badge>
                  {a.policy === "listed" && a.stored && (
                    <Badge>{t(lang, "access.listedN", { n: formatCount(lang, a.listed) })}</Badge>
                  )}
                  {a.default_for_registered && <Badge tone="accent">{t(lang, "access.isDefault")}</Badge>}
                  {!a.connected && <Badge tone="muted">{t(lang, "access.notReported")}</Badge>}
                </div>
              </div>
              <button
                type="button"
                aria-label={`${t(lang, "access.manage")} ${a.label ?? a.id}`}
                onClick={() => {
                  setEditing(a);
                }}
              >
                {t(lang, "access.manage")}
              </button>
            </li>
          ))}
        </ul>
      )}
      {editing !== null && <AccessEditor lang={lang} access={editing} onDone={onChanged} onClose={close} />}
    </Panel>
  );
}
