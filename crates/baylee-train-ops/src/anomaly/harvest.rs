//! Appending new entries to the running list
//! (`~/baylee-data/v6/train/bug-harvest.md`).
//!
//! The list is a markdown table: date, run, kind, seed and deal, what,
//! status. Only `new` entries go there. One row stands for one kind at one
//! place (a seed and deal), however many runs met it: a deal that never
//! ends under every policy is one bug, not one per run. An entry with no
//! place (a panic in a log) is a row per run and text. A kind with more
//! places than [`ROWS_PER_KIND`] in one harvest is folded into one row. A
//! row already in the list is not added again, on any later day.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context as _, ensure};

use super::report::place;
use super::{Entry, Kind, Report};

/// A kind with more places than this in one harvest gets one row.
const ROWS_PER_KIND: usize = 10;

fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

/// One row: the runs that met it and the entry they met.
struct Row<'a> {
    kind: Kind,
    at: String,
    runs: Vec<&'a str>,
    entry: &'a Entry,
}

impl Row<'_> {
    /// What finds the row in the list again: its cells after the date,
    /// without the run when it has a place.
    fn key(&self) -> String {
        if self.entry.deal.is_some() || self.entry.seed.is_some() {
            format!("| {} | {} |", self.kind.label(), cell(&self.at))
        } else {
            format!(
                "| {} | {} | {} | {} |",
                self.runs[0],
                self.kind.label(),
                cell(&self.at),
                cell(&self.entry.what)
            )
        }
    }

    fn text(&self, date: &str) -> String {
        let runs = match self.runs.as_slice() {
            [one] => (*one).to_owned(),
            [first, rest @ ..] => format!("{first} +{} more", rest.len()),
            [] => String::new(),
        };
        let what = if self.runs.len() > 1 {
            format!("{} (in {} runs)", self.entry.what, self.runs.len())
        } else {
            self.entry.what.clone()
        };
        format!(
            "| {date} | {} | {} | {} | {} | open |",
            cell(&runs),
            self.kind.label(),
            cell(&self.at),
            cell(&what)
        )
    }
}

/// The rows a harvest of `reports` adds, before any is checked against
/// the list.
fn rows(reports: &[Report]) -> Vec<Row<'_>> {
    let mut grouped: BTreeMap<(Kind, String, String), Row<'_>> = BTreeMap::new();
    for report in reports {
        for entry in report.new_entries() {
            let at = place(entry.deal, entry.seed, entry.table, entry.seat);
            let placed = entry.deal.is_some() || entry.seed.is_some();
            let key = if placed {
                (entry.kind, at.clone(), String::new())
            } else {
                (
                    entry.kind,
                    at.clone(),
                    format!("{}\n{}", report.run, entry.what),
                )
            };
            grouped
                .entry(key)
                .or_insert_with(|| Row {
                    kind: entry.kind,
                    at,
                    runs: Vec::new(),
                    entry,
                })
                .runs
                .push(&report.run);
        }
    }
    let mut by_kind: BTreeMap<Kind, Vec<Row<'_>>> = BTreeMap::new();
    for row in grouped.into_values() {
        by_kind.entry(row.kind).or_default().push(row);
    }
    let mut out = Vec::new();
    for (_, mut rows) in by_kind {
        if rows.len() <= ROWS_PER_KIND {
            out.extend(rows);
            continue;
        }
        let count = rows.len();
        let first: Vec<String> = rows.iter().take(3).map(|r| r.at.clone()).collect();
        let mut row = rows.swap_remove(0);
        row.at = format!("{count} places, first {}", first.join("; "));
        out.push(row);
    }
    out
}

/// Appends the new rows of `reports` to the list at `path`; returns how
/// many.
///
/// # Errors
/// A list that cannot be read, has no table, or cannot be written.
pub fn append(path: &Path, reports: &[Report], date: &str) -> anyhow::Result<usize> {
    let mut list = fs::read_to_string(path).with_context(|| format!("{}", path.display()))?;
    ensure!(
        list.contains("| Date | Run | Kind |"),
        "{} has no harvest table",
        path.display()
    );
    let mut added = 0;
    for row in rows(reports) {
        let key = row.key();
        if list.lines().any(|line| line.contains(&key)) {
            continue;
        }
        if !list.ends_with('\n') {
            list.push('\n');
        }
        list.push_str(&row.text(date));
        list.push('\n');
        added += 1;
    }
    if added > 0 {
        fs::write(path, list)?;
    }
    Ok(added)
}
