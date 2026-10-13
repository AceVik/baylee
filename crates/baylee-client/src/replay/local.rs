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
/// percent and stopping when `cancel` is set. Nothing is written to disk.
///
/// # Errors
/// The record cannot be played again on this build, in words a player may
/// read.
pub fn project(
    record: &[u8],
    names: Vec<String>,
    mut header: Header,
    perspective: Perspective,
    verify: Verify,
    progress: &AtomicU8,
    cancel: &AtomicBool,
) -> Result<Tape, String> {
    use baylee_client_core::replay::{StopLine, StopReason, TapeMoment};
    use baylee_gamehost::replay::{self as projection, Stop};
    use std::ops::ControlFlow;
    use std::sync::atomic::Ordering;
    let summary = projection::summary(record).map_err(|e| format!("{e:?}"))?;
    header.recorded_by.clone_from(&summary.build);
    let inputs = summary.inputs.max(1);
    let mut moments = Vec::new();
    let made = projection::project(
        record,
        names,
        match perspective {
            Perspective::Seat(seat) => projection::Perspective::Seat(seat),
            Perspective::Public => projection::Perspective::Public,
            Perspective::AllHands(seat) => projection::Perspective::AllHands(seat),
        },
        match verify {
            Verify::Stop => projection::Verify::Stop,
            Verify::Note => projection::Verify::Note,
        },
        |moment| {
            if let Some(n) = moment.input {
                let percent = (n.saturating_add(1).saturating_mul(100) / inputs).min(100);
                progress.store(percent as u8, Ordering::Relaxed);
            }
            moments.push(TapeMoment {
                input: moment.input,
                at: moment.at,
                check: moment.check,
                frames: moment.frames,
            });
            if cancel.load(Ordering::Relaxed) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    )
    .map_err(|e| format!("{e:?}"))?;
    let stop = StopLine {
        stop: match made.stop {
            Stop::End => StopReason::End,
            Stop::Cut => StopReason::Cut,
            Stop::Diverged { .. } => StopReason::Diverged,
            Stop::Refused { .. } => StopReason::Refused,
            Stop::Cancelled => StopReason::Cancelled,
        },
        inputs: made.inputs,
        exact_to: made.exact_to,
        seen_to: made.seen_to,
        diverged: made.diverged,
    };
    Tape::from_moments(header, moments, stop).map_err(|e| e.to_string())
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

/// What the list shows of a record, read without playing it
/// (`baylee_gamehost::replay::summary`), in the list's words.
#[must_use]
pub fn summary(record: &[u8]) -> Option<LocalSummary> {
    use baylee_gamehost::replay::Controller;
    let read = baylee_gamehost::replay::summary(record).ok()?;
    // Whether the record carries the public table's digest: its header
    // does from the release that wrote it on (`record::seen`).
    let checked = record
        .split(|b| *b == b'\n')
        .next()
        .and_then(|first| serde_json::from_slice::<serde_json::Value>(first).ok())
        .is_some_and(|header| header.get("seen").is_some());
    Some(LocalSummary {
        started: read.started,
        ended: (read.ended > 0).then_some(read.ended),
        complete: read.complete,
        winners: read.winners,
        turn: read.turn,
        build: read.build,
        checked,
        seats: read
            .seats
            .into_iter()
            .map(|seat| LocalSeat {
                house: match seat.controller {
                    Controller::House(level) => Some(level),
                    Controller::You | Controller::Player => None,
                },
                colours: seat.colours,
                commander: seat.commander.map(|c| c.get()),
            })
            .collect(),
    })
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
            let summary = summary(&record)?;
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
    let bytes = std::fs::read(path).map_err(|e| format!("{e:?}"))?;
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
