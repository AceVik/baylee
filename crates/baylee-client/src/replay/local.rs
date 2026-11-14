//! A kept record played again here (§3.1): the projection a local replay
//! runs on its worker, the Replays list's rows for the kept records, and the
//! files a developer opens by hand (`BAYLEE_REPLAY`).
//!
//! The kept records are this computer's own (`crate::records`); nothing here
//! writes one, and a re-run is never written to disk (`docs/privacy.md`).

use baylee_client_core::replay::Perspectives;
use baylee_client_core::replay::library::{Entry, LocalSeat, LocalSummary};

pub use super::projection::project;
use baylee_gamehost::record::Line;

/// The folder the kept records live in (`crate::records`).
const FOLDER: &str = "records";
/// The archived ones, under it: out of the list, out of the retention's
/// reach (it trims the folder's own files only), there to be brought back.
const ARCHIVE: &str = "archive";

/// The kept records' folder, and its archive.
fn folders() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let folder = crate::settings::store::folder(FOLDER)?;
    let archive = folder.join(ARCHIVE);
    Some((folder, archive))
}

/// A kept record's file, inflated. `None`: no such file, or not a record.
#[must_use]
pub fn read(file: &str) -> Option<Vec<u8>> {
    if !baylee_client_core::bugreport::is_record_file_name(file) {
        return None;
    }
    let (folder, archive) = folders()?;
    let gzip = crate::records::file::read(&folder.join(file))
        .or_else(|_| crate::records::file::read(&archive.join(file)))
        .ok()?;
    baylee_client_core::bugreport::inflate(&gzip)
}

/// Archives a kept record, or brings it back (the Replays list's Archive
/// and Unarchive): the file moves between the folder and its archive.
/// Nothing is deleted. Only a file the records' own name pattern names.
///
/// # Errors
/// The move failed, in the file system's words.
pub fn archive(file: &str, archived: bool) -> Result<(), String> {
    if !baylee_client_core::bugreport::is_record_file_name(file) {
        return Err("not a kept record".to_string());
    }
    let (folder, archive) = folders().ok_or("no settings folder")?;
    let (from, to) = if archived {
        (folder.join(file), archive.join(file))
    } else {
        (archive.join(file), folder.join(file))
    };
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::rename(from, to).map_err(|e| e.to_string())
}

/// The preset a record was built from: its header's.
#[must_use]
pub fn preset(record: &[u8]) -> Option<baylee_core::preset::GamePreset> {
    let first = record.split(|b| *b == b'\n').next()?;
    match serde_json::from_slice::<Line>(first).ok()? {
        Line::Header { preset, .. } => Some(*preset),
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
                commander: seat.commander.map(baylee_core::ids::CardIndex::get),
            })
            .collect(),
    })
}

/// Every kept record as a list entry, newest first. `build` is this
/// client's (a record it wrote itself is exact), `you` what the player is
/// called in a row.
#[must_use]
pub fn entries(build: &str, you: &str) -> Vec<Entry> {
    let Some((folder, archive)) = folders() else {
        return Vec::new();
    };
    let names = |dir: &std::path::Path| -> Vec<String> {
        std::fs::read_dir(dir).map_or_default(|d| {
            d.filter_map(Result::ok)
                .filter_map(|file| file.file_name().into_string().ok())
                .filter(|name| baylee_client_core::bugreport::is_record_file_name(name))
                .collect()
        })
    };
    let kept = names(&folder).into_iter().map(|n| (n, false));
    let archived = names(&archive).into_iter().map(|n| (n, true));
    let mut entries: Vec<Entry> = kept
        .chain(archived)
        .filter_map(|(name, archived)| {
            let record = read(&name)?;
            let summary = summary(&record)?;
            let mut entry = Entry::of_record(&name, &summary, build, you);
            entry.archived = archived;
            Some(entry)
        })
        .collect();
    entries.sort_by_key(|e| std::cmp::Reverse(e.started));
    entries
}

/// A file opened by hand (`BAYLEE_REPLAY`): a record (`game-*.jsonl.gz`,
/// every perspective of every seat) or a tape.
///
/// # Errors
/// The file cannot be read, or is neither.
pub fn open_file(path: &std::path::Path) -> Result<(super::ReplaySource, Perspectives), String> {
    let bytes = crate::records::file::read(path).map_err(|e| format!("{e:?}"))?;
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
