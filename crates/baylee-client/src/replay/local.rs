//! A kept record played again here (§3.1): the projection a local replay
//! runs on its worker, the Replays list's rows for the kept records, and the
//! files a developer opens by hand (`BAYLEE_REPLAY`).
//!
//! The kept records are this computer's own (`crate::records`); nothing here
//! writes one, and a re-run is never written to disk (`docs/privacy.md`).

use std::sync::atomic::{AtomicBool, AtomicU8};

use baylee_client_core::replay::library::{Entry, LocalSeat, LocalSummary};
use baylee_client_core::replay::{Header, Perspective, Perspectives, Tape, Verify};
use baylee_gamehost::record::Line;

/// The folder the kept records live in (`crate::records`).
const FOLDER: &str = "records";

/// Plays `record` (JSON Lines, inflated) again from `perspective` and reads
/// what the perspective is shown into a tape, reporting progress in
/// percent and stopping when `cancel` is set.
///
/// # Errors
/// The record cannot be played again on this build, in words a player may
/// read.
pub fn project(
    record: &[u8],
    names: Vec<String>,
    header: Header,
    perspective: Perspective,
    verify: Verify,
    progress: &AtomicU8,
    cancel: &AtomicBool,
) -> Result<Tape, String> {
    let _ = (record, names, header, perspective, verify, progress, cancel);
    Err("replaying a game against the house needs this build's projection (lane A)".to_string())
}

/// A kept record's file, inflated. `None`: no such file, or not a record.
#[must_use]
pub fn read(file: &str) -> Option<Vec<u8>> {
    if !baylee_client_core::bugreport::is_record_file_name(file) {
        return None;
    }
    let path = crate::settings::store::folder(FOLDER)?.join(file);
    let gzip = std::fs::read(path).ok()?;
    baylee_client_core::bugreport::inflate(&gzip)
}

/// Deletes a kept record (the Replays list's Delete…). Only a file the
/// records' own name pattern names, and only in their folder.
pub fn delete(file: &str) {
    if !baylee_client_core::bugreport::is_record_file_name(file) {
        return;
    }
    if let Some(folder) = crate::settings::store::folder(FOLDER) {
        let _ = std::fs::remove_file(folder.join(file));
    }
}

/// The preset a record was built from: its header's.
#[must_use]
pub fn preset(record: &[u8]) -> Option<baylee_core::preset::GamePreset> {
    let first = record.split(|b| *b == b'\n').next()?;
    match serde_json::from_slice::<Line>(first).ok()? {
        Line::Header { preset, .. } => Some(preset),
        _ => None,
    }
}

/// What the list shows of a record, read without playing it.
///
/// Interim: `baylee_gamehost::replay::summary` (lane A) reads the same and
/// more (colours, commanders, the turn); this reads the header and the end.
#[must_use]
pub fn summary(file: &str, record: &[u8]) -> Option<LocalSummary> {
    let mut lines = record.split(|b| *b == b'\n').filter(|l| !l.is_empty());
    let (build, preset) = match serde_json::from_slice::<Line>(lines.next()?).ok()? {
        Line::Header { build, preset, .. } => (build, preset),
        _ => return None,
    };
    let started = file
        .strip_prefix("game-")
        .and_then(|rest| rest.get(..13))
        .and_then(|ms| ms.parse().ok())
        .unwrap_or(0);
    let mut summary = LocalSummary {
        started,
        build,
        seats: preset
            .seats
            .iter()
            .map(|seat| LocalSeat {
                house: match &seat.controller {
                    baylee_core::preset::SeatController::Ai(profile) => Some(
                        baylee_core::preset::AIProfile::NAMED
                            .iter()
                            .find(|(_, known)| known == profile)
                            .map_or_else(String::new, |(name, _)| (*name).to_string()),
                    ),
                    _ => None,
                },
                colours: String::new(),
                commander: seat.commanders.first().map(|c| c.card.get()),
            })
            .collect(),
        ..LocalSummary::default()
    };
    for line in lines {
        let Ok(line) = serde_json::from_slice::<Line>(line) else {
            continue;
        };
        match line {
            Line::Input { at, .. } | Line::Chair { at, .. } | Line::DeclaredMind { at, .. } => {
                if at > 0 {
                    summary.ended = Some(at);
                }
            }
            Line::End { at, winners, .. } => {
                summary.complete = true;
                summary.winners = winners;
                if at > 0 {
                    summary.ended = Some(at);
                }
            }
            Line::Header { .. } => {}
        }
    }
    Some(summary)
}

/// Every kept record as a list entry, newest first. `build` is this
/// client's (a record it wrote itself is exact), `you` what the player is
/// called in a row.
#[must_use]
pub fn entries(build: &str, you: &str) -> Vec<Entry> {
    let Some(folder) = crate::settings::store::folder(FOLDER) else {
        return Vec::new();
    };
    let Ok(dir) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = dir
        .filter_map(Result::ok)
        .filter_map(|file| file.file_name().into_string().ok())
        .filter(|name| baylee_client_core::bugreport::is_record_file_name(name))
        .filter_map(|name| {
            let record = read(&name)?;
            let summary = summary(&name, &record)?;
            Some(Entry::of_record(&name, &summary, build, you))
        })
        .collect();
    entries.sort_by(|a, b| b.started.cmp(&a.started));
    entries
}

/// A file opened by hand (`BAYLEE_REPLAY`): a record (`game-*.jsonl.gz`,
/// every perspective of every seat) or a tape.
///
/// # Errors
/// The file cannot be read, or is neither.
pub fn open_file(path: &std::path::Path) -> Result<(super::ReplaySource, Perspectives), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let text = baylee_client_core::bugreport::inflate(&bytes)
        .or_else(|| (bytes.first() == Some(&b'{')).then(|| bytes.clone()))
        .ok_or("not a gzip of JSON Lines")?;
    let first = text.split(|b| *b == b'\n').next().unwrap_or_default();
    if serde_json::from_slice::<serde_json::Value>(first)
        .ok()
        .and_then(|v| v.get("kind").and_then(|k| k.as_str()).map(str::to_owned))
        .as_deref()
        == Some("tape")
    {
        let gzip = if bytes.first() == Some(&b'{') {
            baylee_client_core::bugreport::gzip(&bytes)
        } else {
            bytes
        };
        return Ok((
            super::ReplaySource::Tape {
                gzip: std::sync::Arc::new(gzip),
            },
            Perspectives::default(),
        ));
    }
    let preset = preset(&text).ok_or("neither a record nor a tape")?;
    let names = preset
        .seats
        .iter()
        .enumerate()
        .map(|(i, seat)| match &seat.controller {
            baylee_core::preset::SeatController::Ai(_) => format!("House {i}"),
            _ => format!("Seat {i}"),
        })
        .collect();
    let file = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("record")
        .to_string();
    Ok((
        super::ReplaySource::Record {
            bytes: std::sync::Arc::new(text),
            names,
            file,
            by_hand: true,
        },
        Perspectives::LOCAL,
    ))
}
