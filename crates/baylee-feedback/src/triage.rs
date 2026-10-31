//! Triage (`docs/feedback.md` §"Triage"): the reports as the token API and
//! the UI read and change them.
//!
//! One filter for every list (the list itself, its facets, crash groups,
//! neighbours, CSV): several statuses, kinds and priorities at once, an
//! assignee or none, a label, a crash signature, the trash, and what this
//! admin has not opened yet. A change names what it changes and may name the
//! revision it was made against; another revision is refused (`409`), so two
//! admins never overwrite each other unseen. Every change of a report's
//! state is an audit line; notes are rows of their own. Deleting in the UI
//! moves a report into the trash, from where it can be restored for
//! [`TRASH_DAYS`] days; the token API's delete is final, as it always was.

use axum::http::StatusCode;
use axum::response::Json;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait as _, Value,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Refusal, Status, audit, db_down, refuse};

/// How long a report stays in the trash before the sweep purges it.
pub const TRASH_DAYS: i32 = 30;

/// The most reports one bulk change touches.
pub const MAX_BULK: usize = 200;

/// The most rows a CSV export holds.
pub const MAX_CSV_ROWS: u32 = 5000;

/// The most labels a report carries (the table's bound).
pub const MAX_LABELS: usize = 16;

/// The longest note, in characters (the table's bound).
pub const MAX_NOTE_CHARS: usize = 4000;

/// The most views one admin saves.
pub const MAX_VIEWS: i64 = 50;

/// How urgent a report is.
///
/// @description How urgent a report is: `low`, `normal`, `high` or
/// `critical`.
/// @openapi
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    /// When there is time.
    Low,
    /// In its turn.
    Normal,
    /// Before the others.
    High,
    /// Now: players lose games or data to it.
    Critical,
}

impl Priority {
    const fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "low" => Self::Low,
            "normal" => Self::Normal,
            "high" => Self::High,
            "critical" => Self::Critical,
            _ => return None,
        })
    }
}

/// A status a report is done in: `done_at` is set when it enters one.
const DONE: [&str; 3] = ["resolved", "wont_fix", "duplicate"];

fn status_named(text: &str) -> Option<Status> {
    serde_json::from_value(serde_json::Value::String(text.to_owned())).ok()
}

fn kind_named(text: &str) -> bool {
    matches!(
        text,
        "bug" | "improvement" | "feedback" | "crash" | "other" | "chat"
    )
}

/// A label: a lowercase word, digits and dashes, 1 to 32 characters.
fn valid_label(label: &str) -> bool {
    let mut chars = label.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && label.len() <= 32
        && label
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// What the list is sorted by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Sort {
    #[default]
    Received,
    Changed,
    Priority,
    Status,
    Kind,
    Reporter,
    Gateway,
}

impl Sort {
    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "received" => Self::Received,
            "changed" => Self::Changed,
            "priority" => Self::Priority,
            "status" => Self::Status,
            "kind" => Self::Kind,
            "reporter" => Self::Reporter,
            "gateway" => Self::Gateway,
            _ => return None,
        })
    }

    const fn expression(self) -> &'static str {
        match self {
            // Qualified: a bare name would be the summary's text column.
            Self::Received => "feedback_report.created_at",
            Self::Changed => "feedback_report.updated_at",
            Self::Priority => {
                "CASE priority WHEN 'critical' THEN 4 WHEN 'high' THEN 3 \
                 WHEN 'normal' THEN 2 WHEN 'low' THEN 1 ELSE 0 END"
            }
            Self::Status => {
                "CASE status WHEN 'new' THEN 0 WHEN 'triaged' THEN 1 WHEN 'in_progress' THEN 2 \
                 WHEN 'resolved' THEN 3 WHEN 'wont_fix' THEN 4 ELSE 5 END"
            }
            Self::Kind => "feedback_report.kind",
            Self::Reporter => "feedback_report.reporter",
            Self::Gateway => "feedback_report.gateway",
        }
    }
}

/// The list's filters, all optional; the token API and the UI read the
/// same ones. `status`, `kind` and `priority` take several values,
/// comma-separated (`priority=none` is a report without one); `assignee` is
/// an admin's name or `none`.
///
/// @description The list's filters, all optional; a malformed one is
/// `400`.
/// @openapi
#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct Filter {
    /// @description Statuses, comma-separated: `new`, `triaged`,
    /// `in_progress`, `resolved`, `wont_fix`, `duplicate`.
    status: Option<String>,
    /// @description Kinds, comma-separated: `bug`, `improvement`, `feedback`,
    /// `crash`, `other`, `chat`.
    kind: Option<String>,
    /// @description The gateway's name the reports are filed under
    /// (`(direct)` for direct reports).
    gateway: Option<String>,
    /// @description One reporter's pseudonym.
    reporter: Option<String>,
    /// @description One game's id.
    game_id: Option<String>,
    /// @description Words of the text or of a crash's message, a piece of
    /// the text (`%` and `_` literal), or a piece of a report's id (six hex
    /// digits or more); at most 200 characters.
    /// @internal
    /// Words of the text or of a crash's message, a substring of the text,
    /// or a piece of a report's id.
    q: Option<String>,
    /// @description The first day, `YYYY-MM-DD` in UTC, inclusive.
    /// @openapi format: date
    from: Option<String>,
    /// @description The last day, `YYYY-MM-DD` in UTC, inclusive.
    /// @openapi format: date
    to: Option<String>,
    /// @description Only reports with a record (`true`) or without one
    /// (`false`).
    has_record: Option<bool>,
    /// @description How the reports came: `gateway` or `direct`.
    /// @openapi enum: [gateway, direct]
    channel: Option<String>,
    /// @description What sent the reports, as the client's `source` says
    /// (`llm-seat` for a language-model seat): a source, `none` for
    /// reports without one, or either after `!` for all the others.
    /// 1–64 characters, no control character.
    source: Option<String>,
    /// @description An admin's name, or `none` for reports nobody looks
    /// after.
    assignee: Option<String>,
    /// @description One label: lowercase letters, digits and dashes, 1–32.
    label: Option<String>,
    /// @description Priorities, comma-separated: `critical`, `high`,
    /// `normal`, `low`, or `none` for a report without one.
    priority: Option<String>,
    /// @description One crash signature: 16 hex digits.
    signature: Option<String>,
    /// @description The duplicates of this report (its id).
    /// @openapi format: uuid
    duplicate_of: Option<String>,
    /// @description The trash instead of the reports.
    trash: Option<bool>,
    /// @description Only what the signed-in admin has not opened yet (the
    /// web UI's routes only; the token routes have no admin).
    /// @internal
    /// Only what the signed-in admin has not opened (the UI only).
    unseen: Option<bool>,
    /// @description What the list is sorted by: `received` (the default),
    /// `changed`, `priority`, `status`, `kind`, `reporter` or `gateway`.
    /// @openapi enum: [received, changed, priority, status, kind, reporter, gateway]
    sort: Option<String>,
    /// @description `desc` (the default) or `asc`.
    /// @openapi enum: [desc, asc]
    dir: Option<String>,
    /// @description How many reports a page holds: 50 by default; a value
    /// below 1 or above 200 is taken as 1 or 200.
    /// @openapi default: 50
    pub(crate) limit: Option<u64>,
    /// @description How many reports of the list to skip.
    pub(crate) offset: Option<u64>,
}

/// A dimension the facets count; a facet is counted over the filter
/// without its own dimension, so picking one value still shows the others.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dimension {
    Status,
    Kind,
    Gateway,
    Reporter,
    Assignee,
    Priority,
    Label,
    Channel,
    Source,
}

/// A report's source as the list reads it: what the client's `source` says,
/// cut to 64 characters, so the filter, the facet and the summary agree on
/// one value whatever a client sent.
const SOURCE: &str = "left(feedback_report.client ->> 'source', 64)";

/// The source filter's value: a source or `none`, with `!` before it for
/// all the others; `None` when it is malformed.
fn source_filter(text: &str) -> Option<(bool, Option<&str>)> {
    let (exclude, name) = text
        .strip_prefix('!')
        .map_or((false, text), |rest| (true, rest));
    let length = name.chars().count();
    if !(1..=64).contains(&length) || name.chars().any(char::is_control) {
        return None;
    }
    Some((exclude, (name != "none").then_some(name)))
}

/// A `WHERE` clause being built. `$1` is always the viewing admin (or
/// `NULL`), which the summary's `seen` reads.
struct Clauses {
    sql: Vec<String>,
    values: Vec<Value>,
}

impl Clauses {
    fn new(viewer: Option<Uuid>) -> Self {
        Self {
            sql: Vec::new(),
            values: vec![viewer.into()],
        }
    }

    fn param(&mut self, value: Value) -> String {
        self.values.push(value);
        format!("${}", self.values.len())
    }

    fn add(&mut self, clause: &str, value: Value) {
        let at = self.param(value);
        self.sql.push(clause.replace('?', &at));
    }

    fn any_of(&mut self, column: &str, values: &[String]) {
        let params: Vec<String> = values
            .iter()
            .map(|v| self.param(v.clone().into()))
            .collect();
        self.sql
            .push(format!("{column} IN ({})", params.join(", ")));
    }

    fn clause(&self) -> String {
        if self.sql.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", self.sql.join(" AND "))
        }
    }
}

fn malformed() -> Refusal {
    refuse(StatusCode::BAD_REQUEST, "a filter is malformed")
}

fn list_of(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect()
}

/// A piece of an id (at least six of its characters), as the UI's search
/// box may hold one: the UI shows an id's last eight, its random part, since
/// a v7 id's first ones are its time and alike for reports of one minute.
fn id_fragment(q: &str) -> Option<String> {
    let q = q.trim().to_ascii_lowercase();
    (q.len() >= 6 && q.len() <= 36 && q.chars().all(|c| c.is_ascii_hexdigit() || c == '-'))
        .then_some(q)
}

fn conditions(
    filter: &Filter,
    viewer: Option<Uuid>,
    without: Option<Dimension>,
) -> Result<Clauses, Refusal> {
    let mut c = Clauses::new(viewer);
    if filter.trash == Some(true) {
        c.sql.push("deleted_at IS NOT NULL".to_owned());
    } else {
        c.sql.push("deleted_at IS NULL".to_owned());
    }
    faceted(&mut c, filter, without)?;
    narrowing(&mut c, filter)?;
    Ok(c)
}

/// The clauses of the dimensions the facets count, but `without`.
fn faceted(c: &mut Clauses, filter: &Filter, without: Option<Dimension>) -> Result<(), Refusal> {
    let keep = |d: Dimension| without != Some(d);
    if let Some(status) = filter.status.as_deref().filter(|_| keep(Dimension::Status)) {
        let statuses = list_of(status);
        if statuses.is_empty() || statuses.iter().any(|s| status_named(s).is_none()) {
            return Err(malformed());
        }
        c.any_of("status", &statuses);
    }
    if let Some(kind) = filter.kind.as_deref().filter(|_| keep(Dimension::Kind)) {
        let kinds = list_of(kind);
        if kinds.is_empty() || !kinds.iter().all(|k| kind_named(k)) {
            return Err(malformed());
        }
        c.any_of("kind", &kinds);
    }
    if let Some(gateway) = filter.gateway.clone().filter(|_| keep(Dimension::Gateway)) {
        c.add("gateway = ?", gateway.into());
    }
    if let Some(reporter) = filter
        .reporter
        .clone()
        .filter(|_| keep(Dimension::Reporter))
    {
        c.add("reporter = ?", reporter.into());
    }
    if let Some(channel) = filter
        .channel
        .as_deref()
        .filter(|_| keep(Dimension::Channel))
    {
        if !matches!(channel, "gateway" | "direct") {
            return Err(malformed());
        }
        c.add("channel = ?", channel.to_owned().into());
    }
    if let Some(source) = filter.source.as_deref().filter(|_| keep(Dimension::Source)) {
        match source_filter(source).ok_or_else(malformed)? {
            (false, None) => c.sql.push(format!("{SOURCE} IS NULL")),
            (true, None) => c.sql.push(format!("{SOURCE} IS NOT NULL")),
            (false, Some(name)) => c.add(&format!("{SOURCE} = ?"), name.to_owned().into()),
            // A report without a source is one of the others too.
            (true, Some(name)) => {
                c.add(
                    &format!("{SOURCE} IS DISTINCT FROM ?"),
                    name.to_owned().into(),
                );
            }
        }
    }
    if let Some(assignee) = filter
        .assignee
        .as_deref()
        .filter(|_| keep(Dimension::Assignee))
    {
        if assignee == "none" {
            c.sql.push("assignee IS NULL".to_owned());
        } else {
            c.add("assignee = ?", assignee.to_owned().into());
        }
    }
    if let Some(label) = filter.label.as_deref().filter(|_| keep(Dimension::Label)) {
        if !valid_label(label) {
            return Err(malformed());
        }
        c.add("? = ANY (labels)", label.to_owned().into());
    }
    if let Some(priority) = filter
        .priority
        .as_deref()
        .filter(|_| keep(Dimension::Priority))
    {
        let wanted = list_of(priority);
        if wanted.is_empty()
            || !wanted
                .iter()
                .all(|p| p == "none" || Priority::parse(p).is_some())
        {
            return Err(malformed());
        }
        let named: Vec<String> = wanted.iter().filter(|p| *p != "none").cloned().collect();
        let none = wanted.iter().any(|p| p == "none");
        let params: Vec<String> = named.iter().map(|p| c.param(p.clone().into())).collect();
        let mut parts = Vec::new();
        if !params.is_empty() {
            parts.push(format!("priority IN ({})", params.join(", ")));
        }
        if none {
            parts.push("priority IS NULL".to_owned());
        }
        c.sql.push(format!("({})", parts.join(" OR ")));
    }
    Ok(())
}

/// The clauses no facet counts.
fn narrowing(c: &mut Clauses, filter: &Filter) -> Result<(), Refusal> {
    if let Some(game) = filter.game_id.clone() {
        c.add("game_id = ?", game.into());
    }
    if let Some(signature) = filter.signature.as_deref() {
        if signature.len() != 16 || !signature.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(malformed());
        }
        c.add("signature = ?", signature.to_ascii_lowercase().into());
    }
    if let Some(original) = filter.duplicate_of.as_deref() {
        let original = Uuid::parse_str(original).map_err(|_| malformed())?;
        c.add("duplicate_of = ?", original.into());
    }
    if filter.unseen == Some(true) {
        c.sql.push(
            "NOT EXISTS (SELECT 1 FROM feedback_seen s \
             WHERE s.report_id = feedback_report.id AND s.admin_id = $1)"
                .to_owned(),
        );
    }
    if let Some(q) = filter.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        if q.chars().count() > 200 {
            return Err(malformed());
        }
        if let Some(fragment) = id_fragment(q) {
            let id = c.param(format!("%{fragment}%").into());
            let words = c.param(q.to_owned().into());
            let substring = c.param(crate::contains_pattern(q).into());
            c.sql.push(format!(
                "(id::text LIKE {id} OR search @@ websearch_to_tsquery('simple', {words}) \
                 OR text ILIKE {substring} ESCAPE '\\')"
            ));
        } else {
            let words = c.param(q.to_owned().into());
            let substring = c.param(crate::contains_pattern(q).into());
            c.sql.push(format!(
                "(search @@ websearch_to_tsquery('simple', {words}) \
                 OR text ILIKE {substring} ESCAPE '\\')"
            ));
        }
    }
    if let Some(from) = filter.from.as_deref() {
        let from = crate::day(from).ok_or_else(malformed)?;
        c.add(
            "created_at >= (?::date)::timestamp AT TIME ZONE 'UTC'",
            from.to_string().into(),
        );
    }
    if let Some(to) = filter.to.as_deref() {
        let to = crate::day(to).ok_or_else(malformed)?;
        c.add(
            "created_at < (?::date + 1)::timestamp AT TIME ZONE 'UTC'",
            to.to_string().into(),
        );
    }
    match filter.has_record {
        Some(true) => c.sql.push("record IS NOT NULL".to_owned()),
        Some(false) => c.sql.push("record IS NULL".to_owned()),
        None => {}
    }
    Ok(())
}

fn order(filter: &Filter) -> Result<String, Refusal> {
    let sort = match filter.sort.as_deref() {
        None | Some("") => Sort::default(),
        Some(text) => Sort::parse(text).ok_or_else(malformed)?,
    };
    let dir = match filter.dir.as_deref() {
        None | Some("" | "desc") => "DESC",
        Some("asc") => "ASC",
        Some(_) => return Err(malformed()),
    };
    Ok(format!(
        "{} {dir}, feedback_report.created_at DESC, feedback_report.id DESC",
        sort.expression()
    ))
}

/// The columns a summary shows. `$1` is the viewing admin.
pub(crate) const SUMMARY: &str = "id::text AS id, \
    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS created_at, \
    to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at, \
    gateway, gateway_name, gateway_url, gateway_version, reporter, kind, status, text, game_id, \
    record IS NOT NULL AS has_record, record_complete, \
    coalesce(octet_length(record), 0)::bigint AS record_bytes, issue_number, \
    channel, record_origin, left(feedback_report.client ->> 'source', 64) AS source, \
    assignee, priority, array_to_json(labels)::text AS labels, \
    duplicate_of::text AS duplicate_of, signature, revision, \
    to_char(done_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS done_at, \
    to_char(deleted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS deleted_at, \
    EXISTS (SELECT 1 FROM feedback_seen s WHERE s.report_id = feedback_report.id \
        AND s.admin_id = $1) AS seen, \
    (SELECT count(*) FROM feedback_note n WHERE n.report_id = feedback_report.id) AS notes";

/// A report as the read API shows it.
///
/// @description A report as a list shows it.
/// @openapi
#[derive(Serialize)]
pub(crate) struct Summary {
    /// @description Its id, a `UUIDv7`.
    /// @openapi format: uuid
    id: String,
    /// @description When it came, RFC 3339 in UTC.
    /// @openapi format: date-time
    created_at: String,
    /// @description When it last changed, RFC 3339 in UTC.
    /// @openapi format: date-time
    updated_at: String,
    /// @description The name of the gateway it came through, as this
    /// service configures that gateway's token; `(direct)` for a direct
    /// report.
    gateway: String,
    /// @description The name the gateway gave itself, if it sent one.
    gateway_name: Option<String>,
    /// @description Where the gateway said it is, if it sent that.
    gateway_url: Option<String>,
    /// @description The gateway's build, or a direct report's client build.
    gateway_version: String,
    /// @description The reporter's pseudonym: the gateway's HMAC, or for a
    /// direct report this service's of the device's id. Never an account.
    reporter: String,
    /// @description What it is about.
    /// @openapi enum: [bug, improvement, feedback, crash, other, chat]
    kind: String,
    /// @description Where it stands.
    /// @openapi
    /// ```yaml
    /// enum: [new, triaged, in_progress, resolved, wont_fix, duplicate]
    /// ```
    status: String,
    /// @description What the player wrote.
    text: String,
    /// @description The game it was written at, if any.
    game_id: Option<String>,
    /// @description Whether a game record is attached.
    has_record: bool,
    /// @description Whether the record reaches the game's end; `null`
    /// without a record.
    record_complete: Option<bool>,
    /// @description The record's size in bytes, compressed; 0 without one.
    record_bytes: i64,
    /// @description How it came: `gateway` (passed on by one) or `direct` (a
    /// client sent it itself, unauthenticated).
    /// @internal
    /// How it came: `gateway` (passed on by one) or `direct` (a client
    /// sent it itself, unauthenticated).
    /// @openapi enum: [gateway, direct]
    channel: String,
    /// @description Who wrote the record: `gateway`, or `client`
    /// (unverified); `null` without one.
    /// @internal
    /// Who wrote the record: `gateway`, or `client` (unverified); `None`
    /// without one.
    /// @openapi enum: [gateway, client, null]
    record_origin: Option<String>,
    /// @description What sent it, as the client's `source` says, at most 64
    /// characters (`llm-seat` for a language-model seat); `null` without
    /// one. The client's word, unchecked.
    /// @internal
    /// What sent it, as the client's `source` says, cut as [`SOURCE`] cuts it.
    source: Option<String>,
    /// @description The GitHub issue it is linked to, by number.
    /// @internal
    /// The GitHub issue it is linked to, in [`crate::ISSUE_REPOSITORY`].
    issue_number: Option<i32>,
    /// @description That issue's page, made from the number; `null` without
    /// one.
    /// @internal
    /// That issue's page, made here from the number rather than stored, so
    /// no link the UI renders was ever typed by anyone.
    issue_url: Option<String>,
    /// @description The admin who looks after it, by name.
    /// @internal
    /// The admin who looks after it.
    assignee: Option<String>,
    /// @description How urgent it is; `null` for none.
    /// @openapi enum: [critical, high, normal, low, null]
    priority: Option<String>,
    /// @description Its labels, sorted.
    labels: Vec<String>,
    /// @description The report this one duplicates.
    /// @internal
    /// The report this one duplicates.
    /// @openapi format: uuid
    duplicate_of: Option<String>,
    /// @description A crash's signature: the same crash has the same one.
    /// @internal
    /// A crash's signature: the same crash has the same one.
    signature: Option<String>,
    /// @description Moved on by every change; a change may name the one it
    /// was made against.
    /// @internal
    /// Moved on by every change; a change may name the one it was made
    /// against.
    revision: i64,
    /// @description When it entered a done status (`resolved`, `wont_fix`,
    /// `duplicate`), RFC 3339 in UTC.
    /// @internal
    /// When it entered a done status (resolved, won't fix, duplicate).
    /// @openapi format: date-time
    done_at: Option<String>,
    /// @description When it went into the trash, RFC 3339 in UTC.
    /// @internal
    /// When it went into the trash.
    /// @openapi format: date-time
    deleted_at: Option<String>,
    /// @description Whether the signed-in admin has opened it; always
    /// `false` over the tokens.
    /// @internal
    /// Whether the signed-in admin has opened it (always false for the
    /// token API).
    seen: bool,
    /// @description How many internal notes it has.
    /// @internal
    /// How many internal notes it has.
    notes: i64,
    /// @description What the client said about itself; only on one report,
    /// never in a list.
    #[serde(skip_serializing_if = "Option::is_none")]
    client: Option<serde_json::Value>,
}

impl Summary {
    fn of(row: &sea_orm::QueryResult, client: bool) -> Result<Self, sea_orm::DbErr> {
        let issue_number: Option<i32> = row.try_get("", "issue_number")?;
        let labels: String = row.try_get("", "labels")?;
        Ok(Self {
            id: row.try_get("", "id")?,
            created_at: row.try_get("", "created_at")?,
            updated_at: row.try_get("", "updated_at")?,
            gateway: row.try_get("", "gateway")?,
            gateway_name: row.try_get("", "gateway_name")?,
            gateway_url: row.try_get("", "gateway_url")?,
            gateway_version: row.try_get("", "gateway_version")?,
            reporter: row.try_get("", "reporter")?,
            kind: row.try_get("", "kind")?,
            status: row.try_get("", "status")?,
            text: row.try_get("", "text")?,
            game_id: row.try_get("", "game_id")?,
            has_record: row.try_get("", "has_record")?,
            record_complete: row.try_get("", "record_complete")?,
            record_bytes: row.try_get("", "record_bytes")?,
            channel: row.try_get("", "channel")?,
            record_origin: row.try_get("", "record_origin")?,
            source: row.try_get("", "source")?,
            issue_number,
            issue_url: issue_number.map(|n| format!("{}/issues/{n}", crate::ISSUE_REPOSITORY)),
            assignee: row.try_get("", "assignee")?,
            priority: row.try_get("", "priority")?,
            labels: serde_json::from_str(&labels).unwrap_or_default(),
            duplicate_of: row.try_get("", "duplicate_of")?,
            signature: row.try_get("", "signature")?,
            revision: row.try_get("", "revision")?,
            done_at: row.try_get("", "done_at")?,
            deleted_at: row.try_get("", "deleted_at")?,
            seen: row.try_get("", "seen")?,
            notes: row.try_get("", "notes")?,
            client: if client {
                Some(row.try_get("", "client")?)
            } else {
                None
            },
        })
    }
}

/// One page of the list.
/// @openapi
#[derive(Serialize)]
pub(crate) struct Listing {
    /// @description The page's reports.
    reports: Vec<Summary>,
    /// @description How many reports the filter matches in all.
    total: i64,
}

/// One page of the list. The trash is swept first.
pub(crate) async fn listing(
    db: &DatabaseConnection,
    filter: &Filter,
    viewer: Option<Uuid>,
) -> Result<Listing, Refusal> {
    sweep(db).await?;
    let c = conditions(filter, viewer, None)?;
    let clause = c.clause();
    let order = order(filter)?;
    let limit = filter.limit.unwrap_or(50).clamp(1, 200);
    // Past Postgres's `bigint` an offset is an error; far below it, nothing.
    let offset = filter.offset.unwrap_or(0).min(1 << 40);
    let backend = db.get_database_backend();
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            backend,
            format!(
                "SELECT {SUMMARY} FROM feedback_report {clause} \
                 ORDER BY {order} LIMIT {limit} OFFSET {offset}"
            ),
            c.values.clone(),
        ))
        .await
        .map_err(|e| db_down(&e))?;
    let total = db
        .query_one_raw(Statement::from_sql_and_values(
            backend,
            format!("SELECT count(*) AS n FROM feedback_report {clause}"),
            c.values,
        ))
        .await
        .map_err(|e| db_down(&e))?
        .map_or(Ok(0), |row| row.try_get::<i64>("", "n"))
        .map_err(|e| db_down(&e))?;
    let reports = rows
        .iter()
        .map(|row| Summary::of(row, false))
        .collect::<Result<_, _>>()
        .map_err(|e| db_down(&e))?;
    Ok(Listing { reports, total })
}

/// One internal note.
/// @openapi
#[derive(Serialize)]
pub(crate) struct Note {
    /// @description Its id.
    /// @openapi format: uuid
    id: String,
    /// @description When it was written, RFC 3339 in UTC.
    /// @openapi format: date-time
    at: String,
    /// @description The admin who wrote it.
    author: String,
    /// @description What it says.
    text: String,
}

/// A report with its `client` part and, for the UI, its notes.
///
/// @description One report, with its `client` object and, in the web UI,
/// its notes.
/// @openapi
#[derive(Serialize)]
pub(crate) struct Detail {
    #[serde(flatten)]
    summary: Summary,
    /// @description Its internal notes, oldest first; only in the web UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    note_list: Option<Vec<Note>>,
}

pub(crate) async fn fetch(
    db: &DatabaseConnection,
    id: Uuid,
    viewer: Option<Uuid>,
    with_notes: bool,
) -> Result<Json<Detail>, Refusal> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!("SELECT {SUMMARY}, client FROM feedback_report WHERE id = $2"),
            [viewer.into(), id.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| refuse(StatusCode::NOT_FOUND, "no such report"))?;
    let summary = Summary::of(&row, true).map_err(|e| db_down(&e))?;
    let note_list = if with_notes {
        Some(notes_of(db, id).await?)
    } else {
        None
    };
    Ok(Json(Detail { summary, note_list }))
}

async fn notes_of(db: &DatabaseConnection, id: Uuid) -> Result<Vec<Note>, Refusal> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT id::text AS id, \
                 to_char(at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS at, \
                 author, text FROM feedback_note WHERE report_id = $1 ORDER BY at, id",
            [id.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?;
    rows.iter()
        .map(|row| {
            Ok(Note {
                id: row.try_get("", "id")?,
                at: row.try_get("", "at")?,
                author: row.try_get("", "author")?,
                text: row.try_get("", "text")?,
            })
        })
        .collect::<Result<_, sea_orm::DbErr>>()
        .map_err(|e| db_down(&e))
}

/// A field a change may leave alone (left out), clear (`null`) or set.
#[derive(Clone, Debug, Default)]
pub(crate) enum Edit<T> {
    #[default]
    Keep,
    Clear,
    Set(T),
}

impl<T> Edit<T> {
    const fn touched(&self) -> bool {
        !matches!(self, Self::Keep)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Edit<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Option::<T>::deserialize(d)?.map_or(Self::Clear, Self::Set))
    }
}

/// What an admin may change about a report. A part left out stays as it
/// is; `null` clears an issue, an assignee, a priority or a duplicate.
///
/// @description What to change about a report. A part left out stays as
/// it is; `null` clears an issue, an assignee, a priority or a duplicate.
/// At least one part besides `expected_revision`.
/// @openapi
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Change {
    /// @description The status it gets. Marking it a duplicate sets
    /// `duplicate`; unmarking one sets `triaged` unless a status is named.
    #[serde(default)]
    pub(crate) status: Option<Status>,
    /// @description The GitHub issue it is linked to, by number (1 or more),
    /// or `null` for none.
    /// @openapi type: [integer, 'null']
    /// @openapi minimum: 1
    #[serde(default)]
    pub(crate) issue: Edit<u32>,
    /// @description The admin who looks after it, by name, or `null` for
    /// nobody.
    /// @openapi type: [string, 'null']
    #[serde(default)]
    assignee: Edit<String>,
    /// @description How urgent it is, or `null` for none.
    /// @openapi type: [string, 'null']
    /// @openapi enum: [critical, high, normal, low, null]
    #[serde(default)]
    priority: Edit<Priority>,
    /// @description Its labels, the whole set: at most 16, each lowercase
    /// letters, digits and dashes, 1–32 characters.
    /// @internal
    /// The whole set; for the bulk route, `labels_add` and `labels_remove`.
    #[serde(default)]
    labels: Option<Vec<String>>,
    /// @description The report it duplicates (an original, not itself a
    /// duplicate), or `null` for none.
    /// @openapi type: [string, 'null']
    /// @openapi format: uuid
    #[serde(default)]
    duplicate_of: Edit<Uuid>,
    /// @description The revision the change was made against; another one
    /// is `409`.
    /// @internal
    /// The revision the change was made against; another one is `409`.
    #[serde(default)]
    expected_revision: Option<i64>,
}

impl Change {
    pub(crate) fn status(status: Status) -> Self {
        Self {
            status: Some(status),
            ..Self::default()
        }
    }

    fn is_empty(&self) -> bool {
        self.status.is_none()
            && !self.issue.touched()
            && !self.assignee.touched()
            && !self.priority.touched()
            && self.labels.is_none()
            && !self.duplicate_of.touched()
    }
}

fn bad(why: &'static str) -> Refusal {
    refuse(StatusCode::BAD_REQUEST, why)
}

/// A label set as stored: checked, deduplicated, sorted.
fn label_set(labels: &[String]) -> Result<Vec<String>, Refusal> {
    let mut set: Vec<String> = labels.iter().map(|l| l.trim().to_owned()).collect();
    if !set.iter().all(|l| valid_label(l)) {
        return Err(bad(
            "a label is lowercase letters, digits and dashes, at most 32",
        ));
    }
    set.sort();
    set.dedup();
    if set.len() > MAX_LABELS {
        return Err(bad("a report carries at most 16 labels"));
    }
    Ok(set)
}

async fn admin_exists(db: &impl ConnectionTrait, name: &str) -> Result<bool, Refusal> {
    Ok(db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT 1 AS one FROM feedback_admin WHERE name = $1",
            [name.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?
        .is_some())
}

/// What [`change_one`] found.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// No such report, or it is in the trash.
    Missing,
    /// The report already was as the change asks.
    Unchanged,
    Changed,
}

/// Adds `value` to a statement's values; its placeholder.
fn push(values: &mut Vec<Value>, value: Value) -> String {
    values.push(value);
    format!("${}", values.len())
}

/// The report as a change finds it, locked for the rest of the
/// transaction.
struct Current {
    revision: i64,
    status: String,
    issue: Option<i32>,
    assignee: Option<String>,
    priority: Option<String>,
    labels: Vec<String>,
    duplicate_of: Option<Uuid>,
}

async fn current(tx: &impl ConnectionTrait, id: Uuid) -> Result<Option<Current>, Refusal> {
    let down = |e: sea_orm::DbErr| db_down(&e);
    let Some(row) = tx
        .query_one_raw(Statement::from_sql_and_values(
            tx.get_database_backend(),
            "SELECT revision, status, issue_number, assignee, priority, \
                 array_to_json(labels)::text AS labels, duplicate_of \
             FROM feedback_report WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
            [id.into()],
        ))
        .await
        .map_err(down)?
    else {
        return Ok(None);
    };
    let labels: String = row.try_get("", "labels").map_err(down)?;
    Ok(Some(Current {
        revision: row.try_get("", "revision").map_err(down)?,
        status: row.try_get("", "status").map_err(down)?,
        issue: row.try_get("", "issue_number").map_err(down)?,
        assignee: row.try_get("", "assignee").map_err(down)?,
        priority: row.try_get("", "priority").map_err(down)?,
        labels: serde_json::from_str(&labels).unwrap_or_default(),
        duplicate_of: row.try_get("", "duplicate_of").map_err(down)?,
    }))
}

/// `Some(new)` where `new` differs from `old`.
fn differs<T: PartialEq>(old: &T, new: T) -> Option<T> {
    (*old != new).then_some(new)
}

/// What a change does to a report, part by part; `None` leaves a part as
/// it is, `Some(None)` clears it.
#[derive(Default)]
#[allow(clippy::option_option)] // left alone, cleared, set: three states
struct Plan {
    status: Option<&'static str>,
    issue: Option<Option<i32>>,
    assignee: Option<Option<String>>,
    priority: Option<Option<String>>,
    labels: Option<Vec<String>>,
    duplicate_of: Option<Option<Uuid>>,
}

impl Plan {
    const fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.issue.is_none()
            && self.assignee.is_none()
            && self.priority.is_none()
            && self.labels.is_none()
            && self.duplicate_of.is_none()
    }
}

/// What `change` does to report `id` as it is `now`; checked, and only what
/// differs.
async fn plan(
    tx: &impl ConnectionTrait,
    id: Uuid,
    now: &Current,
    change: &Change,
    labels_add: &[String],
    labels_remove: &[String],
) -> Result<Plan, Refusal> {
    let down = |e: sea_orm::DbErr| db_down(&e);
    let issue = match change.issue {
        Edit::Keep => None,
        Edit::Clear => differs(&now.issue, None),
        Edit::Set(n) => differs(
            &now.issue,
            Some(
                i32::try_from(n)
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or_else(|| bad("not an issue number"))?,
            ),
        ),
    };
    let assignee = match &change.assignee {
        Edit::Keep => None,
        Edit::Clear => differs(&now.assignee, None),
        Edit::Set(name) => {
            if !admin_exists(tx, name).await? {
                return Err(bad("no admin of that name"));
            }
            differs(&now.assignee, Some(name.clone()))
        }
    };
    let priority = match change.priority {
        Edit::Keep => None,
        Edit::Clear => differs(&now.priority, None),
        Edit::Set(p) => differs(&now.priority, Some(p.name().to_owned())),
    };
    let labels = {
        let mut set = match &change.labels {
            Some(labels) => label_set(labels)?,
            None => now.labels.clone(),
        };
        set.extend(label_set(labels_add)?);
        let removed = label_set(labels_remove)?;
        set.retain(|l| !removed.contains(l));
        set.sort();
        set.dedup();
        if set.len() > MAX_LABELS {
            return Err(bad("a report carries at most 16 labels"));
        }
        differs(&now.labels, set)
    };
    let mut duplicate_of = match change.duplicate_of {
        Edit::Keep => None,
        Edit::Clear => differs(&now.duplicate_of, None),
        Edit::Set(original) => {
            if original == id {
                return Err(bad("a report does not duplicate itself"));
            }
            // One level only: the original is no duplicate itself.
            let row = tx
                .query_one_raw(Statement::from_sql_and_values(
                    tx.get_database_backend(),
                    "SELECT duplicate_of IS NULL AS original FROM feedback_report \
                     WHERE id = $1 AND deleted_at IS NULL",
                    [original.into()],
                ))
                .await
                .map_err(down)?
                .ok_or_else(|| bad("no report to be a duplicate of"))?;
            if !row.try_get::<bool>("", "original").map_err(down)? {
                return Err(bad("that report is a duplicate itself; name its original"));
            }
            differs(&now.duplicate_of, Some(original))
        }
    };
    // Marked as a duplicate, it is one; no longer one of a report, and
    // with no status named, it goes back to triaged.
    let wanted = change.status.map(Status::name).or(match duplicate_of {
        Some(Some(_)) => Some("duplicate"),
        Some(None) if now.status == "duplicate" => Some("triaged"),
        _ => None,
    });
    let status = wanted.filter(|s| *s != now.status);
    let final_status = status.unwrap_or(now.status.as_str());
    if final_status != "duplicate" && duplicate_of.is_none() && now.duplicate_of.is_some() {
        duplicate_of = Some(None);
    }
    Ok(Plan {
        status,
        issue,
        assignee,
        priority,
        labels,
        duplicate_of,
    })
}

/// Writes `plan` to report `id`, with an audit line per part.
async fn write(
    tx: &impl ConnectionTrait,
    id: Uuid,
    actor: &str,
    plan: Plan,
) -> Result<(), Refusal> {
    let down = |e: sea_orm::DbErr| db_down(&e);
    let Plan {
        status,
        issue,
        assignee,
        priority,
        labels,
        duplicate_of,
    } = plan;
    let mut values: Vec<Value> = vec![id.into()];
    let mut sets = vec![
        "revision = revision + 1".to_owned(),
        "updated_at = now()".to_owned(),
    ];
    if let Some(status) = status {
        let at = push(&mut values, status.into());
        sets.push(format!("status = {at}"));
        sets.push(if DONE.contains(&status) {
            "done_at = coalesce(done_at, now())".to_owned()
        } else {
            "done_at = NULL".to_owned()
        });
    }
    if let Some(issue) = issue {
        let at = push(&mut values, issue.into());
        sets.push(format!("issue_number = {at}"));
    }
    if let Some(assignee) = &assignee {
        let at = push(&mut values, assignee.clone().into());
        sets.push(format!("assignee = {at}"));
    }
    if let Some(priority) = &priority {
        let at = push(&mut values, priority.clone().into());
        sets.push(format!("priority = {at}"));
    }
    if let Some(labels) = &labels {
        let json = serde_json::to_string(labels).unwrap_or_else(|_| "[]".to_owned());
        let at = push(&mut values, json.into());
        sets.push(format!(
            "labels = ARRAY(SELECT jsonb_array_elements_text({at}::jsonb))"
        ));
    }
    if let Some(original) = duplicate_of {
        let at = push(&mut values, original.into());
        sets.push(format!("duplicate_of = {at}"));
    }
    tx.execute_raw(Statement::from_sql_and_values(
        tx.get_database_backend(),
        format!(
            "UPDATE feedback_report SET {} WHERE id = $1",
            sets.join(", ")
        ),
        values,
    ))
    .await
    .map_err(down)?;
    let mut lines: Vec<(&str, Option<String>)> = Vec::new();
    if let Some(status) = status {
        lines.push(("status", Some(status.to_owned())));
    }
    if let Some(issue) = issue {
        lines.push(("issue", issue.map(|n| format!("#{n}"))));
    }
    if let Some(assignee) = assignee {
        lines.push(("assign", assignee));
    }
    if let Some(priority) = priority {
        lines.push(("priority", priority));
    }
    if let Some(labels) = labels {
        lines.push(("labels", Some(labels.join(","))));
    }
    if let Some(original) = duplicate_of {
        lines.push(("duplicate", original.map(|o| o.to_string())));
    }
    for (action, detail) in lines {
        audit(tx, actor, id, action, detail).await.map_err(down)?;
    }
    if let Some(Some(original)) = duplicate_of {
        let moved = tx
            .query_all_raw(Statement::from_sql_and_values(
                tx.get_database_backend(),
                "UPDATE feedback_report SET duplicate_of = $2, revision = revision + 1, \
                     updated_at = now() WHERE duplicate_of = $1 RETURNING id",
                [id.into(), original.into()],
            ))
            .await
            .map_err(down)?;
        for row in moved {
            let other: Uuid = row.try_get("", "id").map_err(down)?;
            audit(tx, actor, other, "duplicate", Some(original.to_string()))
                .await
                .map_err(down)?;
        }
    }
    Ok(())
}

/// Changes report `id` in `tx` as `actor`: only what differs is written,
/// each part with its audit line. A report marked as another's duplicate
/// gets the status `duplicate`, and one no longer marked goes back to
/// `triaged`; one that leaves that status is nobody's duplicate any more;
/// the duplicates of a report that becomes a duplicate move on to its
/// original, so no chain forms.
async fn change_one(
    tx: &impl ConnectionTrait,
    id: Uuid,
    actor: &str,
    change: &Change,
    labels_add: &[String],
    labels_remove: &[String],
) -> Result<Outcome, Refusal> {
    let Some(now) = current(tx, id).await? else {
        return Ok(Outcome::Missing);
    };
    if change.expected_revision.is_some_and(|r| r != now.revision) {
        return Err(refuse(
            StatusCode::CONFLICT,
            "the report changed meanwhile; read it again",
        ));
    }
    let plan = plan(tx, id, &now, change, labels_add, labels_remove).await?;
    if plan.is_empty() {
        return Ok(Outcome::Unchanged);
    }
    write(tx, id, actor, plan).await?;
    Ok(Outcome::Changed)
}

async fn exists(db: &impl ConnectionTrait, id: Uuid) -> Result<bool, Refusal> {
    Ok(db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT 1 AS one FROM feedback_report WHERE id = $1 AND deleted_at IS NULL",
            [id.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?
        .is_some())
}

/// Applies `change` to report `id` as `actor`, in one transaction, and
/// answers the report as it is now.
pub(crate) async fn apply_change(
    db: &DatabaseConnection,
    id: Uuid,
    actor: &str,
    change: Change,
    viewer: Option<Uuid>,
) -> Result<Json<Detail>, Refusal> {
    if change.is_empty() {
        return Err(bad("nothing to change"));
    }
    let down = |e: sea_orm::DbErr| db_down(&e);
    let tx = db.begin().await.map_err(down)?;
    if change_one(&tx, id, actor, &change, &[], &[]).await? == Outcome::Missing {
        return Err(refuse(StatusCode::NOT_FOUND, "no such report"));
    }
    tx.commit().await.map_err(down)?;
    fetch(db, id, viewer, viewer.is_some()).await
}

/// What the bulk route does besides a change.
///
/// @description `trash` (into the trash), `restore` (out of it) or `purge`
/// (deleted for good, only from the trash).
/// @openapi
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Action {
    /// Into the trash.
    Trash,
    /// Out of the trash.
    Restore,
    /// Gone for good; only from the trash.
    Purge,
}

/// `POST /ui/api/reports/bulk`.
///
/// @description One change or one action for many reports: exactly one of
/// `change` and `action`.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bulk {
    /// @description The reports, 1 to 200 after duplicates are dropped.
    /// @openapi
    /// ```yaml
    /// minItems: 1
    /// ```
    ids: Vec<Uuid>,
    /// @description The change for each of them.
    #[serde(default)]
    change: Option<BulkChange>,
    /// @description The action for each of them.
    #[serde(default)]
    action: Option<Action>,
}

/// A change for many reports: as [`Change`], with labels added and removed
/// instead of a whole set, and no revision.
///
/// @description A change for many reports: as one report's, with labels
/// added and removed instead of a whole set, and no revision.
/// @openapi
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BulkChange {
    /// @description The status each gets.
    #[serde(default)]
    status: Option<Status>,
    /// @description The GitHub issue, by number, or `null` for none.
    /// @openapi type: [integer, 'null']
    /// @openapi minimum: 1
    #[serde(default)]
    issue: Edit<u32>,
    /// @description The admin who looks after them, or `null` for nobody.
    /// @openapi type: [string, 'null']
    #[serde(default)]
    assignee: Edit<String>,
    /// @description How urgent they are, or `null` for none.
    /// @openapi type: [string, 'null']
    /// @openapi enum: [critical, high, normal, low, null]
    #[serde(default)]
    priority: Edit<Priority>,
    /// @description Labels each gets.
    #[serde(default)]
    labels_add: Vec<String>,
    /// @description Labels each loses.
    #[serde(default)]
    labels_remove: Vec<String>,
    /// @description The report they duplicate, or `null` for none.
    /// @openapi type: [string, 'null']
    /// @openapi format: uuid
    #[serde(default)]
    duplicate_of: Edit<Uuid>,
}

/// What a bulk request did.
/// @openapi
#[derive(Serialize)]
pub(crate) struct BulkDone {
    /// @description How many reports it changed; one already as asked, or
    /// not where the action needs it, is not counted.
    /// @internal
    /// How many reports it changed.
    changed: usize,
}

/// Many reports at once, in one transaction: all of them or none.
pub(crate) async fn bulk(
    db: &DatabaseConnection,
    actor: &str,
    bulk: Bulk,
) -> Result<Json<BulkDone>, Refusal> {
    let mut ids = bulk.ids;
    ids.sort();
    ids.dedup();
    if ids.is_empty() || ids.len() > MAX_BULK {
        return Err(bad("name between 1 and 200 reports"));
    }
    if bulk.change.is_some() == bulk.action.is_some() {
        return Err(bad("a bulk request is a change or an action"));
    }
    let down = |e: sea_orm::DbErr| db_down(&e);
    let tx = db.begin().await.map_err(down)?;
    let mut changed = 0;
    if let Some(change) = bulk.change {
        let one = Change {
            status: change.status,
            issue: change.issue,
            assignee: change.assignee,
            priority: change.priority,
            labels: None,
            duplicate_of: change.duplicate_of,
            expected_revision: None,
        };
        if one.is_empty() && change.labels_add.is_empty() && change.labels_remove.is_empty() {
            return Err(bad("nothing to change"));
        }
        for id in &ids {
            if change_one(
                &tx,
                *id,
                actor,
                &one,
                &change.labels_add,
                &change.labels_remove,
            )
            .await?
                == Outcome::Changed
            {
                changed += 1;
            }
        }
    } else if let Some(action) = bulk.action {
        for id in &ids {
            if act(&tx, *id, actor, action).await? {
                changed += 1;
            }
        }
    }
    tx.commit().await.map_err(down)?;
    Ok(Json(BulkDone { changed }))
}

/// Moves one report into or out of the trash, or purges it from there.
/// `Ok(false)` when it is not where the action needs it.
pub(crate) async fn act(
    tx: &impl ConnectionTrait,
    id: Uuid,
    actor: &str,
    action: Action,
) -> Result<bool, Refusal> {
    let down = |e: sea_orm::DbErr| db_down(&e);
    let (sql, line) = match action {
        Action::Trash => (
            "UPDATE feedback_report SET deleted_at = now(), revision = revision + 1, \
                 updated_at = now() WHERE id = $1 AND deleted_at IS NULL",
            "trash",
        ),
        Action::Restore => (
            "UPDATE feedback_report SET deleted_at = NULL, revision = revision + 1, \
                 updated_at = now() WHERE id = $1 AND deleted_at IS NOT NULL",
            "restore",
        ),
        Action::Purge => (
            "DELETE FROM feedback_report WHERE id = $1 AND deleted_at IS NOT NULL",
            "delete",
        ),
    };
    let done = tx
        .execute_raw(Statement::from_sql_and_values(
            tx.get_database_backend(),
            sql,
            [id.into()],
        ))
        .await
        .map_err(down)?;
    if done.rows_affected() == 0 {
        return Ok(false);
    }
    audit(tx, actor, id, line, None).await.map_err(down)?;
    if action == Action::Purge {
        tracing::info!(report = %id, actor, "report deleted");
    }
    Ok(true)
}

/// One action on one report, in its own transaction.
pub(crate) async fn act_on(
    db: &DatabaseConnection,
    id: Uuid,
    actor: &str,
    action: Action,
) -> Result<(), Refusal> {
    let down = |e: sea_orm::DbErr| db_down(&e);
    let tx = db.begin().await.map_err(down)?;
    if !act(&tx, id, actor, action).await? {
        return Err(refuse(StatusCode::NOT_FOUND, "no such report there"));
    }
    tx.commit().await.map_err(down)
}

/// Purges what lay in the trash longer than [`TRASH_DAYS`], with an audit
/// line each (actor `sweep`). Run before the list is read.
pub(crate) async fn sweep(db: &DatabaseConnection) -> Result<(), Refusal> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM feedback_report \
             WHERE deleted_at < now() - make_interval(days => $1) RETURNING id",
            [TRASH_DAYS.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?;
    for row in rows {
        let id: Uuid = row.try_get("", "id").map_err(|e| db_down(&e))?;
        audit(db, "sweep", id, "delete", None)
            .await
            .map_err(|e| db_down(&e))?;
    }
    Ok(())
}

/// One value of a facet and how many reports have it; `value` is `None`
/// for an unassigned report or one without a priority.
///
/// @description One value and how many reports have it.
/// @openapi
#[derive(Serialize)]
pub(crate) struct Count {
    /// @description The value; `null` for nobody assigned or no priority.
    value: Option<String>,
    /// @description How many reports have it.
    count: i64,
}

/// The counts beside the list, each over the active filter without its
/// own dimension.
///
/// @description The counts beside the list, most first, each over the
/// filter without its own dimension.
/// @openapi
#[derive(Serialize)]
pub(crate) struct Facets {
    /// @description Per status.
    statuses: Vec<Count>,
    /// @description Per kind.
    kinds: Vec<Count>,
    /// @description Per gateway.
    gateways: Vec<Count>,
    /// @description The 50 reporters with the most reports.
    /// @internal
    /// The reporters with the most reports, at most [`TOP_REPORTERS`].
    reporters: Vec<Count>,
    /// @description Per assignee.
    assignees: Vec<Count>,
    /// @description Per priority.
    priorities: Vec<Count>,
    /// @description Per label.
    labels: Vec<Count>,
    /// @description Per channel.
    channels: Vec<Count>,
    /// @description Per source; `null` for reports without one.
    sources: Vec<Count>,
    /// @description What the whole filter matches.
    /// @internal
    /// What the whole filter matches.
    total: i64,
}

/// How many reporters the facets name.
pub const TOP_REPORTERS: u32 = 50;

async fn facet(
    db: &DatabaseConnection,
    filter: &Filter,
    viewer: Option<Uuid>,
    dimension: Dimension,
) -> Result<Vec<Count>, Refusal> {
    let c = conditions(filter, viewer, Some(dimension))?;
    let clause = c.clause();
    let (value, from, limit) = match dimension {
        Dimension::Status => ("status", "feedback_report", 100),
        Dimension::Kind => ("kind", "feedback_report", 100),
        Dimension::Gateway => ("gateway", "feedback_report", 1000),
        Dimension::Reporter => ("reporter", "feedback_report", TOP_REPORTERS),
        Dimension::Assignee => ("assignee", "feedback_report", 1000),
        Dimension::Priority => ("priority", "feedback_report", 100),
        Dimension::Channel => ("channel", "feedback_report", 10),
        Dimension::Source => (SOURCE, "feedback_report", 100),
        Dimension::Label => ("l", "feedback_report, unnest(labels) l", 200),
    };
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!(
                "SELECT {value} AS value, count(*) AS count FROM {from} {clause} \
                 GROUP BY {value} ORDER BY count(*) DESC, {value} NULLS FIRST LIMIT {limit}"
            ),
            c.values,
        ))
        .await
        .map_err(|e| db_down(&e))?;
    rows.iter()
        .map(|row| {
            Ok(Count {
                value: row.try_get("", "value")?,
                count: row.try_get("", "count")?,
            })
        })
        .collect::<Result<_, sea_orm::DbErr>>()
        .map_err(|e| db_down(&e))
}

pub(crate) async fn facets(
    db: &DatabaseConnection,
    filter: &Filter,
    viewer: Option<Uuid>,
) -> Result<Facets, Refusal> {
    let c = conditions(filter, viewer, None)?;
    let total = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!("SELECT count(*) AS n FROM feedback_report {}", c.clause()),
            c.values,
        ))
        .await
        .map_err(|e| db_down(&e))?
        .map_or(Ok(0), |row| row.try_get::<i64>("", "n"))
        .map_err(|e| db_down(&e))?;
    Ok(Facets {
        statuses: facet(db, filter, viewer, Dimension::Status).await?,
        kinds: facet(db, filter, viewer, Dimension::Kind).await?,
        gateways: facet(db, filter, viewer, Dimension::Gateway).await?,
        reporters: facet(db, filter, viewer, Dimension::Reporter).await?,
        assignees: facet(db, filter, viewer, Dimension::Assignee).await?,
        priorities: facet(db, filter, viewer, Dimension::Priority).await?,
        labels: facet(db, filter, viewer, Dimension::Label).await?,
        channels: facet(db, filter, viewer, Dimension::Channel).await?,
        sources: facet(db, filter, viewer, Dimension::Source).await?,
        total,
    })
}

/// One crash, by its signature, under the filter.
/// @openapi
#[derive(Serialize)]
pub(crate) struct CrashGroup {
    /// @description The crash's signature.
    signature: String,
    /// @description How many reports of it the filter matches.
    count: i64,
    /// @description Of them, how many are not done.
    /// @internal
    /// Of them, how many are not done.
    open: i64,
    /// @description When the first came, RFC 3339 in UTC.
    /// @openapi format: date-time
    first: String,
    /// @description When the last came, RFC 3339 in UTC.
    /// @openapi format: date-time
    last: String,
    /// @description The newest report of it, by id.
    /// @internal
    /// The newest report of it.
    /// @openapi format: uuid
    latest: String,
    /// @description Its message, as the newest report has it.
    /// @internal
    /// Its message, as the newest report has it.
    message: Option<String>,
    /// @description Where it happened, as the newest report has it.
    location: Option<String>,
}

/// The crashes the filter matches, grouped by signature, most reports first.
pub(crate) async fn crash_groups(
    db: &DatabaseConnection,
    filter: &Filter,
    viewer: Option<Uuid>,
) -> Result<Vec<CrashGroup>, Refusal> {
    let mut c = conditions(filter, viewer, None)?;
    c.sql.push("signature IS NOT NULL".to_owned());
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!(
                "SELECT signature, count(*) AS count, \
                     count(*) FILTER (WHERE status NOT IN ('resolved', 'wont_fix', 'duplicate')) \
                         AS open, \
                     to_char(min(created_at) AT TIME ZONE 'UTC', \
                         'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS first, \
                     to_char(max(created_at) AT TIME ZONE 'UTC', \
                         'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS last, \
                     (array_agg(id::text ORDER BY created_at DESC))[1] AS latest, \
                     (array_agg(client -> 'crash' ->> 'message' ORDER BY created_at DESC))[1] \
                         AS message, \
                     (array_agg(client -> 'crash' ->> 'location' ORDER BY created_at DESC))[1] \
                         AS location \
                 FROM feedback_report {} GROUP BY signature \
                 ORDER BY count(*) DESC, max(created_at) DESC LIMIT 100",
                c.clause()
            ),
            c.values,
        ))
        .await
        .map_err(|e| db_down(&e))?;
    rows.iter()
        .map(|row| {
            Ok(CrashGroup {
                signature: row.try_get("", "signature")?,
                count: row.try_get("", "count")?,
                open: row.try_get("", "open")?,
                first: row.try_get("", "first")?,
                last: row.try_get("", "last")?,
                latest: row.try_get("", "latest")?,
                message: row.try_get("", "message")?,
                location: row.try_get("", "location")?,
            })
        })
        .collect::<Result<_, sea_orm::DbErr>>()
        .map_err(|e| db_down(&e))
}

/// Where a report stands in the list the filter makes.
/// @openapi
#[derive(Serialize)]
pub(crate) struct Neighbours {
    /// @description The report above it, by id, if any.
    /// @internal
    /// The one above it, if any.
    /// @openapi format: uuid
    previous: Option<String>,
    /// @description The report below it, by id, if any.
    /// @internal
    /// The one below it, if any.
    /// @openapi format: uuid
    next: Option<String>,
    /// @description Its place, from 1; `null` when the filter does not match
    /// it.
    /// @internal
    /// Its place, from 1; `None` when the filter does not match it.
    position: Option<i64>,
    /// @description How many reports the filter matches; 0 when it does not
    /// match this one.
    total: i64,
}

pub(crate) async fn neighbours(
    db: &DatabaseConnection,
    id: Uuid,
    filter: &Filter,
    viewer: Option<Uuid>,
) -> Result<Neighbours, Refusal> {
    let mut c = conditions(filter, viewer, None)?;
    let order = order(filter)?;
    let target = c.param(id.into());
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!(
                "WITH ordered AS (SELECT id, \
                     lag(id) OVER (ORDER BY {order}) AS previous, \
                     lead(id) OVER (ORDER BY {order}) AS next, \
                     row_number() OVER (ORDER BY {order}) AS position, \
                     count(*) OVER () AS total \
                 FROM feedback_report {}) \
                 SELECT previous::text AS previous, next::text AS next, position, total \
                 FROM ordered WHERE id = {target}",
                c.clause()
            ),
            c.values,
        ))
        .await
        .map_err(|e| db_down(&e))?;
    match row {
        Some(row) => Ok(Neighbours {
            previous: row.try_get("", "previous").map_err(|e| db_down(&e))?,
            next: row.try_get("", "next").map_err(|e| db_down(&e))?,
            position: row.try_get("", "position").map_err(|e| db_down(&e))?,
            total: row.try_get("", "total").map_err(|e| db_down(&e))?,
        }),
        None => Ok(Neighbours {
            previous: None,
            next: None,
            position: None,
            total: 0,
        }),
    }
}

/// A cell as a spreadsheet may read it: quoted, and a leading `=`, `+`,
/// `-`, `@`, tab or carriage return defused with an apostrophe, so no cell
/// is ever read as a formula.
fn cell(text: &str) -> String {
    let defused = if text.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{text}")
    } else {
        text.to_owned()
    };
    format!("\"{}\"", defused.replace('"', "\"\""))
}

/// The filter's reports as CSV, at most [`MAX_CSV_ROWS`], newest first by
/// the filter's order. The `client` part stays out but for its build.
pub(crate) async fn csv(
    db: &DatabaseConnection,
    filter: &Filter,
    viewer: Option<Uuid>,
) -> Result<String, Refusal> {
    let c = conditions(filter, viewer, None)?;
    let order = order(filter)?;
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!(
                "SELECT {SUMMARY}, coalesce(client -> 'build' ->> 'version', '') AS build \
                 FROM feedback_report {} ORDER BY {order} LIMIT {MAX_CSV_ROWS}",
                c.clause()
            ),
            c.values,
        ))
        .await
        .map_err(|e| db_down(&e))?;
    let mut out = String::from(
        "id,received,changed,kind,status,priority,assignee,labels,gateway,gateway_version,\
         client_build,channel,reporter,game_id,has_record,record_origin,source,issue,\
         duplicate_of,signature,done,text\r\n",
    );
    for row in &rows {
        let summary = Summary::of(row, false).map_err(|e| db_down(&e))?;
        let build: String = row.try_get("", "build").map_err(|e| db_down(&e))?;
        let fields = [
            summary.id,
            summary.created_at,
            summary.updated_at,
            summary.kind,
            summary.status,
            summary.priority.unwrap_or_default(),
            summary.assignee.unwrap_or_default(),
            summary.labels.join(" "),
            summary.gateway,
            summary.gateway_version,
            build,
            summary.channel,
            summary.reporter,
            summary.game_id.unwrap_or_default(),
            summary.has_record.to_string(),
            summary.record_origin.unwrap_or_default(),
            summary.source.unwrap_or_default(),
            summary.issue_url.unwrap_or_default(),
            summary.duplicate_of.unwrap_or_default(),
            summary.signature.unwrap_or_default(),
            summary.done_at.unwrap_or_default(),
            summary.text,
        ];
        let line: Vec<String> = fields.iter().map(|f| cell(f)).collect();
        out.push_str(&line.join(","));
        out.push_str("\r\n");
    }
    Ok(out)
}

/// Adds a note to report `id` as `author`.
pub(crate) async fn add_note(
    db: &DatabaseConnection,
    id: Uuid,
    author: &str,
    text: &str,
) -> Result<Json<Vec<Note>>, Refusal> {
    let text = text.trim();
    if text.is_empty() || text.chars().count() > MAX_NOTE_CHARS {
        return Err(bad("a note holds 1 to 4000 characters"));
    }
    if !exists(db, id).await? {
        return Err(refuse(StatusCode::NOT_FOUND, "no such report"));
    }
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO feedback_note (id, report_id, author, text) VALUES ($1, $2, $3, $4)",
        [
            Uuid::now_v7().into(),
            id.into(),
            author.into(),
            text.to_owned().into(),
        ],
    ))
    .await
    .map_err(|e| db_down(&e))?;
    notes_of(db, id).await.map(Json)
}

/// Removes a note; only its author may.
pub(crate) async fn remove_note(
    db: &DatabaseConnection,
    id: Uuid,
    note: Uuid,
    author: &str,
) -> Result<Json<Vec<Note>>, Refusal> {
    let done = db
        .execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM feedback_note WHERE id = $1 AND report_id = $2 AND author = $3",
            [note.into(), id.into(), author.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?;
    if done.rows_affected() == 0 {
        return Err(refuse(StatusCode::NOT_FOUND, "no note of yours there"));
    }
    notes_of(db, id).await.map(Json)
}

/// Marks reports as opened by `admin`, or as not opened.
pub(crate) async fn mark_seen(
    db: &DatabaseConnection,
    admin: Uuid,
    ids: &[Uuid],
    seen: bool,
) -> Result<(), Refusal> {
    if ids.is_empty() || ids.len() > 500 {
        return Err(bad("name between 1 and 500 reports"));
    }
    let backend = db.get_database_backend();
    let list = serde_json::to_string(&ids.iter().map(Uuid::to_string).collect::<Vec<_>>())
        .unwrap_or_default();
    let sql = if seen {
        "INSERT INTO feedback_seen (admin_id, report_id) \
         SELECT $1, r.id FROM feedback_report r \
         WHERE r.id IN (SELECT jsonb_array_elements_text($2::jsonb)::uuid) \
         ON CONFLICT DO NOTHING"
    } else {
        "DELETE FROM feedback_seen WHERE admin_id = $1 \
         AND report_id IN (SELECT jsonb_array_elements_text($2::jsonb)::uuid)"
    };
    db.execute_raw(Statement::from_sql_and_values(
        backend,
        sql,
        [admin.into(), list.into()],
    ))
    .await
    .map_err(|e| db_down(&e))?;
    Ok(())
}

/// One saved view: a name and the list's query string.
/// @openapi
#[derive(Serialize)]
pub(crate) struct View {
    /// @description Its id.
    /// @openapi format: uuid
    id: String,
    /// @description Its name.
    name: String,
    /// @description The list's query string, without `?`.
    query: String,
}

pub(crate) async fn views(db: &DatabaseConnection, admin: Uuid) -> Result<Vec<View>, Refusal> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT id::text AS id, name, query FROM feedback_view \
             WHERE admin_id = $1 ORDER BY name",
            [admin.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?;
    rows.iter()
        .map(|row| {
            Ok(View {
                id: row.try_get("", "id")?,
                name: row.try_get("", "name")?,
                query: row.try_get("", "query")?,
            })
        })
        .collect::<Result<_, sea_orm::DbErr>>()
        .map_err(|e| db_down(&e))
}

/// Saves a view, replacing one of the same name; the query must be one the
/// list takes.
pub(crate) async fn save_view(
    db: &DatabaseConnection,
    admin: Uuid,
    name: &str,
    query: &str,
) -> Result<Vec<View>, Refusal> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 60 || name.chars().any(char::is_control) {
        return Err(bad("a view's name holds 1 to 60 characters"));
    }
    let query = query.trim_start_matches('?');
    if query.len() > 2000 {
        return Err(bad("a view's query is at most 2000 characters"));
    }
    let filter: Filter = filter_of(query).ok_or_else(malformed)?;
    conditions(&filter, None, None)?;
    order(&filter)?;
    let count = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT count(*) AS n FROM feedback_view WHERE admin_id = $1 AND name <> $2",
            [admin.into(), name.to_owned().into()],
        ))
        .await
        .map_err(|e| db_down(&e))?
        .map_or(Ok(0), |row| row.try_get::<i64>("", "n"))
        .map_err(|e| db_down(&e))?;
    if count >= MAX_VIEWS {
        return Err(bad("an admin keeps at most 50 views"));
    }
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO feedback_view (id, admin_id, name, query) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (admin_id, name) DO UPDATE SET query = excluded.query",
        [
            Uuid::now_v7().into(),
            admin.into(),
            name.to_owned().into(),
            query.to_owned().into(),
        ],
    ))
    .await
    .map_err(|e| db_down(&e))?;
    views(db, admin).await
}

/// The query string as the list would read it.
fn filter_of(query: &str) -> Option<Filter> {
    axum::extract::Query::<Filter>::try_from_uri(
        &format!("/?{query}").parse::<axum::http::Uri>().ok()?,
    )
    .ok()
    .map(|q| q.0)
}

pub(crate) async fn remove_view(
    db: &DatabaseConnection,
    admin: Uuid,
    view: Uuid,
) -> Result<Vec<View>, Refusal> {
    let done = db
        .execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM feedback_view WHERE id = $1 AND admin_id = $2",
            [view.into(), admin.into()],
        ))
        .await
        .map_err(|e| db_down(&e))?;
    if done.rows_affected() == 0 {
        return Err(refuse(StatusCode::NOT_FOUND, "no view of yours there"));
    }
    views(db, admin).await
}

/// Every admin's name, for assigning.
pub(crate) async fn admins(db: &DatabaseConnection) -> Result<Vec<String>, Refusal> {
    let rows = db
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT name FROM feedback_admin ORDER BY name",
        ))
        .await
        .map_err(|e| db_down(&e))?;
    rows.iter()
        .map(|row| row.try_get("", "name"))
        .collect::<Result<_, sea_orm::DbErr>>()
        .map_err(|e| db_down(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(query: &str) -> Filter {
        filter_of(query).expect("a filter")
    }

    #[test]
    fn a_label_is_a_lowercase_word() {
        for good in ["ui", "crash-2", "a", "0day"] {
            assert!(valid_label(good), "{good}");
        }
        for bad in ["", "UI", "-ui", "u i", "ü", &"a".repeat(33)] {
            assert!(!valid_label(bad), "{bad}");
        }
        assert_eq!(
            label_set(&["b".into(), "a".into(), "b".into()]).ok(),
            Some(vec!["a".into(), "b".into()])
        );
        assert!(label_set(&(0..17).map(|i| format!("l{i}")).collect::<Vec<_>>()).is_err());
    }

    #[test]
    fn a_cell_is_never_a_formula() {
        assert_eq!(cell("=HYPERLINK(1)"), "\"'=HYPERLINK(1)\"");
        assert_eq!(cell("@SUM"), "\"'@SUM\"");
        assert_eq!(cell("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(cell("plain"), "\"plain\"");
    }

    #[test]
    fn several_statuses_kinds_and_priorities_filter_at_once() {
        let c = conditions(
            &filter("status=new,triaged&kind=bug&priority=high,none&assignee=none"),
            None,
            None,
        )
        .ok()
        .expect("a filter");
        let sql = c.sql.join(" AND ");
        assert!(sql.contains("status IN ($2, $3)"), "{sql}");
        assert!(sql.contains("kind IN ($4)"), "{sql}");
        assert!(sql.contains("assignee IS NULL"), "{sql}");
        assert!(
            sql.contains("(priority IN ($5) OR priority IS NULL)"),
            "{sql}"
        );
        assert!(
            sql.starts_with("deleted_at IS NULL"),
            "the trash stays out: {sql}"
        );
    }

    #[test]
    fn a_facet_leaves_out_its_own_dimension() {
        let c = conditions(
            &filter("status=new&kind=bug"),
            None,
            Some(Dimension::Status),
        )
        .ok()
        .expect("a filter");
        let sql = c.sql.join(" AND ");
        assert!(!sql.contains("status"), "{sql}");
        assert!(sql.contains("kind"), "{sql}");
    }

    #[test]
    fn a_malformed_filter_is_refused() {
        for query in [
            "status=open",
            "kind=bugs",
            "priority=urgent",
            "label=Bad%20Label",
            "signature=xyz",
            "duplicate_of=not-a-uuid",
            "channel=mail",
            "from=2026-13-01",
            "sort=size",
            "dir=up",
        ] {
            let f = filter(query);
            assert!(
                conditions(&f, None, None).is_err() || order(&f).is_err(),
                "{query}"
            );
        }
    }

    #[test]
    fn the_search_box_takes_a_piece_of_an_id() {
        assert_eq!(id_fragment("0192F3"), Some("0192f3".into()));
        assert_eq!(id_fragment("crash"), None);
        assert_eq!(id_fragment("abc"), None, "too short to be meant as an id");
        let c = conditions(&filter("q=0192f3ab"), None, None).ok().unwrap();
        assert!(c.sql.join(" ").contains("id::text LIKE"));
    }
}
