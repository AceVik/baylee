//! The report's two files: `anomalies-<run>.json` and a short markdown.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use super::{Class, Report};

/// How many new entries the markdown lists; the JSON has every one.
const SHOWN: usize = 20;

/// The short markdown of a report.
#[must_use]
pub fn markdown(report: &Report) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Anomalies: {}\n", report.run);
    let _ = writeln!(
        out,
        "{} result `{}`, {} games{}.\n",
        report.source,
        report.result.display(),
        report.games,
        report
            .build
            .as_ref()
            .map_or_else(String::new, |build| format!(", build {build}"))
    );
    let new = report.new_entries().count();
    let _ = writeln!(
        out,
        "{new} new, {} known, {} expected entries (`anomalies-{}.json` lists all).\n",
        report
            .entries
            .iter()
            .filter(|e| e.class == Class::Known)
            .count(),
        report
            .entries
            .iter()
            .filter(|e| e.class == Class::Expected)
            .count(),
        report.run
    );
    if !report.counts.is_empty() {
        let _ = writeln!(out, "| Kind | Count |\n| --- | --- |");
        for (kind, count) in report.counts.iter().filter(|(k, _)| !k.contains('/')) {
            let _ = writeln!(out, "| {kind} | {count} |");
        }
        out.push('\n');
    }
    if new > 0 {
        let _ = writeln!(out, "## New\n");
        for entry in report.new_entries().take(SHOWN) {
            let _ = writeln!(
                out,
                "- **{}** {}{}: {}{}",
                entry.kind.label(),
                place(entry.deal, entry.seed, entry.table, entry.seat),
                if entry.count > 1 {
                    format!(" ({}x)", entry.count)
                } else {
                    String::new()
                },
                entry.what,
                entry
                    .reproduce
                    .as_ref()
                    .map_or_else(String::new, |cmd| format!("\n  `{cmd}`"))
            );
        }
        if new > SHOWN {
            let _ = writeln!(out, "- … and {} more", new - SHOWN);
        }
        out.push('\n');
    }
    replay_section(&mut out, report);
    counter_section(&mut out, report);
    if !report.not_recorded.is_empty() {
        let _ = writeln!(out, "## Not recorded\n");
        for note in &report.not_recorded {
            let _ = writeln!(out, "- {note}");
        }
    }
    out
}

/// The replay-exact audit's paragraph.
fn replay_section(out: &mut String, report: &Report) {
    let _ = writeln!(out, "## Replay-exact audit\n");
    let replay = &report.replay;
    if replay.status.starts_with("ran") {
        let _ = writeln!(
            out,
            "{}: {} of {} finished games sampled (1 in {}), {} replayed exactly; converter {} \
             (commit {}), records by commit {}.\n",
            replay.status,
            replay.sampled,
            replay.finished,
            replay.every,
            replay.exact,
            replay
                .converter
                .as_ref()
                .map_or_else(|| "?".to_owned(), |p| p.display().to_string()),
            replay.converter_commit.as_deref().unwrap_or("?"),
            replay.writer_commit.as_deref().unwrap_or("?"),
        );
    } else {
        let _ = writeln!(out, "{}.\n", replay.status);
    }
}

/// The counterspell watch's paragraph.
fn counter_section(out: &mut String, report: &Report) {
    let _ = writeln!(out, "## Counterspell watch\n");
    match &report.counters {
        serde_json::Value::String(why) => {
            let _ = writeln!(out, "{why}.\n");
        }
        counted => {
            let _ = writeln!(
                out,
                "{} counterspells by the net's seat in {} replayed games, {} on a poor target \
                 (share {}); by whose {}, by kind {}.\n",
                counted["counterspells"],
                counted["replayed"],
                counted["poor"],
                counted["poor_share"]
                    .as_f64()
                    .map_or_else(|| "-".to_owned(), |s| format!("{:.0}%", s * 100.0)),
                counted["by_whose"],
                counted["by_kind"]
            );
        }
    }
}

/// Where in the run an entry is, as words.
pub(super) fn place(
    deal: Option<u64>,
    seed: Option<u64>,
    table: Option<u64>,
    seat: Option<u64>,
) -> String {
    let mut parts = Vec::new();
    if let Some(seed) = seed {
        parts.push(format!("seed {seed}"));
    }
    if let Some(deal) = deal {
        parts.push(format!("deal {deal}"));
    }
    if let Some(table) = table {
        parts.push(format!("table {table}"));
    }
    if let Some(seat) = seat {
        parts.push(format!("seat {seat}"));
    }
    if parts.is_empty() {
        "the run".to_owned()
    } else {
        parts.join(", ")
    }
}

/// Writes `anomalies-<run>.json` and `.md` into `dir` (the result's own
/// directory unless told otherwise); returns the JSON's path.
///
/// # Errors
/// A file that cannot be written.
pub fn write(report: &Report, dir: &Path) -> anyhow::Result<PathBuf> {
    let json = dir.join(format!("anomalies-{}.json", report.run));
    fs::write(&json, serde_json::to_vec_pretty(report)?)?;
    fs::write(
        dir.join(format!("anomalies-{}.md", report.run)),
        markdown(report),
    )?;
    Ok(json)
}
