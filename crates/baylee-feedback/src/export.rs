//! `baylee-feedback export-reports`: the open reports, their notes and their
//! triage history as JSON Lines, for the one-time copy into Baylee Central
//! (`docs/feedback.md` §"The one-time copy to Central", baylee-central
//! `central-service --import-reports-stdin`).
//!
//! One report a line, oldest first, read one at a time so a record of
//! 64 MiB is held once, its gzip as hex (Central reads that without a
//! decoder of its own); then a last line with the format, the count and the
//! SHA-256 of every line before it, so Central refuses a copy that was cut
//! short. What Central would refuse is brought within its bounds as the
//! gateway brings a report (`baylee_account::reports`); a report it would
//! still refuse (a reporter that is not hex) is left out and named by id.

use std::io::Write;

use anyhow::{Context as _, Result};
use baylee_account::reports::{self as intake, Build, Crash, ReportKind};
use sea_orm::{ConnectionTrait, DatabaseConnection, QueryResult, Statement, Value};
use serde::Serialize;
use sha2::{Digest as _, Sha256};

/// The copy's format, as the last line names it and Central checks.
pub const FORMAT: &str = "baylee-feedback-reports/1";

/// What the gateway said of itself.
#[derive(Serialize)]
struct Says {
    name: Option<String>,
    url: Option<String>,
    version: Option<String>,
}

/// A record, gzip in lowercase hex.
#[derive(Serialize)]
struct Record {
    complete: bool,
    origin: &'static str,
    gzip: String,
}

/// A note, by the name of the admin who wrote it.
#[derive(Serialize)]
struct Note {
    at: i64,
    author: String,
    text: String,
}

/// A line of the report's triage history.
#[derive(Serialize)]
struct Audit {
    at: i64,
    actor: String,
    action: String,
    detail: Option<String>,
}

/// One report, as a line.
#[derive(Serialize)]
struct Line {
    id: String,
    created_at: i64,
    updated_at: i64,
    gateway: String,
    gateway_says: Says,
    channel: String,
    reporter: String,
    kind: ReportKind,
    status: String,
    text: String,
    game_id: Option<String>,
    build: Option<Build>,
    crash: Option<Crash>,
    issue: Option<i32>,
    priority: Option<String>,
    labels: Vec<String>,
    assignee: Option<String>,
    client_gzip: Option<String>,
    record: Option<Record>,
    notes: Vec<Note>,
    audit: Vec<Audit>,
}

/// The last line.
#[derive(Serialize)]
struct Trailer {
    format: &'static str,
    count: u64,
    sha256: String,
}

fn kind(word: &str) -> Option<ReportKind> {
    Some(match word {
        "bug" => ReportKind::Bug,
        "improvement" => ReportKind::Improvement,
        "feedback" => ReportKind::Feedback,
        "crash" => ReportKind::Crash,
        "other" => ReportKind::Other,
        _ => return None,
    })
}

fn get<T: sea_orm::TryGetable>(row: &QueryResult, column: &str) -> Result<T> {
    row.try_get("", column)
        .with_context(|| format!("reading {column}"))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn gzip(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    out.write_all(bytes)?;
    Ok(out.finish()?)
}

const COLUMNS: &str = "id, floor(extract(epoch FROM created_at))::bigint AS created_at, \
     floor(extract(epoch FROM updated_at))::bigint AS updated_at, gateway, gateway_name, \
     gateway_url, gateway_version, channel, reporter, kind, status, text, game_id, \
     client::text AS client, record, record_complete, record_origin, issue_number, priority, \
     array_to_json(labels)::text AS labels, assignee";

/// The report in `row` as a line, with its `notes` and `audit`; or why
/// Central would not take it.
fn line(
    row: &QueryResult,
    notes: Vec<Note>,
    audit: Vec<Audit>,
) -> Result<std::result::Result<Line, &'static str>> {
    let id: sea_orm::prelude::Uuid = get(row, "id")?;
    let kind = kind(&get::<String>(row, "kind")?).context("a kind")?;
    let client: serde_json::Value = serde_json::from_str(&get::<String>(row, "client")?)?;
    let crash = (kind == ReportKind::Crash)
        .then(|| intake::crash_of(&client))
        .flatten();
    let Some(text) = intake::text_of(&get::<String>(row, "text")?, kind, crash.as_ref()) else {
        return Ok(Err("no text"));
    };
    let reporter: String = get(row, "reporter")?;
    if reporter.is_empty()
        || reporter.len() > intake::MAX_REPORTER
        || !reporter.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Ok(Err("a reporter that is not hex"));
    }
    let channel: String = get(row, "channel")?;
    let game_id = get::<Option<String>>(row, "game_id")?.filter(|g| {
        channel == "gateway"
            && !g.is_empty()
            && g.chars().count() <= intake::MAX_GAME_ID
            && g.chars().all(|c| c.is_ascii_graphic())
    });
    let client_gzip = match client.as_object() {
        Some(map) if map.is_empty() => None,
        _ => Some(gzip(&serde_json::to_vec(&client)?)?)
            .filter(|g| g.len() <= intake::MAX_CLIENT_GZIP as usize),
    };
    let origin = if get::<Option<String>>(row, "record_origin")?.as_deref() == Some("client") {
        "client"
    } else {
        "gateway"
    };
    let max = if origin == "client" {
        intake::MAX_CLIENT_RECORD_GZIP
    } else {
        intake::MAX_RECORD_GZIP
    };
    let record = get::<Option<Vec<u8>>>(row, "record")?
        .filter(|r| r.starts_with(&[0x1f, 0x8b]) && r.len() <= max as usize)
        .map(|r| -> Result<Record> {
            Ok(Record {
                complete: get::<Option<bool>>(row, "record_complete")?.unwrap_or(false),
                origin,
                gzip: hex(&r),
            })
        })
        .transpose()?;
    let labels: Vec<String> = serde_json::from_str(&get::<String>(row, "labels")?)?;
    Ok(Ok(Line {
        id: id.simple().to_string(),
        created_at: get(row, "created_at")?,
        updated_at: get(row, "updated_at")?,
        gateway: get(row, "gateway")?,
        gateway_says: Says {
            name: get::<Option<String>>(row, "gateway_name")?
                .as_deref()
                .and_then(intake::said),
            url: get::<Option<String>>(row, "gateway_url")?
                .as_deref()
                .and_then(intake::said),
            version: intake::said(&get::<String>(row, "gateway_version")?),
        },
        channel,
        reporter,
        kind,
        status: get(row, "status")?,
        text,
        game_id,
        build: intake::build_of(&client),
        crash,
        issue: get(row, "issue_number")?,
        priority: get(row, "priority")?,
        labels,
        assignee: get(row, "assignee")?,
        client_gzip: client_gzip.map(|g| hex(&g)),
        record,
        notes,
        audit,
    }))
}

async fn rows(
    db: &DatabaseConnection,
    sql: &str,
    report: sea_orm::prelude::Uuid,
) -> Result<Vec<QueryResult>> {
    Ok(db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            sql,
            [Value::from(report)],
        ))
        .await?)
}

/// Writes every open report outside the trash to `out`, then the trailer;
/// how many were written and how many were left out.
///
/// # Errors
/// The database, or `out`.
pub async fn export_reports(db: &DatabaseConnection, out: &mut impl Write) -> Result<(u64, u64)> {
    let backend = db.get_database_backend();
    let ids = db
        .query_all_raw(Statement::from_string(
            backend,
            // a reported chat message stays at the service: Central takes none
            "SELECT id FROM feedback_report WHERE deleted_at IS NULL AND kind <> 'chat' \
                 AND status IN ('new', 'triaged', 'in_progress') ORDER BY created_at, id",
        ))
        .await?;
    let mut digest = Sha256::new();
    let (mut count, mut left_out) = (0_u64, 0_u64);
    for id in &ids {
        let id: sea_orm::prelude::Uuid = get(id, "id")?;
        let Some(row) = rows(
            db,
            &format!("SELECT {COLUMNS} FROM feedback_report WHERE id = $1"),
            id,
        )
        .await?
        .pop() else {
            continue;
        };
        let notes = rows(db, "SELECT floor(extract(epoch FROM at))::bigint AS at, author, text FROM feedback_note WHERE report_id = $1 ORDER BY at, id", id)
            .await?
            .iter()
            .map(|n| Ok(Note { at: get(n, "at")?, author: get(n, "author")?, text: get(n, "text")? }))
            .collect::<Result<_>>()?;
        let audit = rows(db, "SELECT floor(extract(epoch FROM at))::bigint AS at, actor, action, detail FROM feedback_audit WHERE report_id = $1 ORDER BY at, id", id)
            .await?
            .iter()
            .map(|a| Ok(Audit { at: get(a, "at")?, actor: get(a, "actor")?, action: get(a, "action")?, detail: get(a, "detail")? }))
            .collect::<Result<_>>()?;
        match line(&row, notes, audit)? {
            Ok(line) => {
                let mut bytes = serde_json::to_vec(&line)?;
                bytes.push(b'\n');
                digest.update(&bytes);
                out.write_all(&bytes)?;
                count += 1;
            }
            Err(why) => {
                tracing::warn!(report = %id, why, "left out of the copy: Central would not take it");
                left_out += 1;
            }
        }
    }
    let trailer = Trailer {
        format: FORMAT,
        count,
        sha256: hex(&digest.finalize()),
    };
    serde_json::to_writer(&mut *out, &trailer)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok((count, left_out))
}
