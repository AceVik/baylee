//! The records of games this client hosted itself, kept on disk (#315).
//!
//! A game against the house runs here (`host::LocalHost`), so no gateway
//! keeps its record. The client keeps the last few itself, gzipped, in
//! `records/` beside its settings, bounded by
//! `bugreport::retention` (count and weight). They stay on this machine:
//! a record leaves it only with a report the player ticked it for, and
//! the form reads the one in memory, never these files. In a browser
//! there is no such folder, and nothing is kept.

/// The folder the records live in, beside the settings.
#[cfg(not(target_arch = "wasm32"))]
const FOLDER: &str = "records";

/// Writes `jsonl`, gzipped, as the kept record `name`, then deletes the
/// oldest past the bounds. Nothing in a process that has not opened the
/// settings store (a test), as everything the store writes.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn keep(name: &str, jsonl: &[u8]) {
    use baylee_client_core::bugreport::{gzip, is_record_file_name, retention};
    if !is_record_file_name(name) {
        return;
    }
    let Some(folder) = crate::settings::store::folder(FOLDER) else {
        return;
    };
    crate::settings::store::write_at(&folder.join(name), gzip(jsonl));
    let Ok(entries) = std::fs::read_dir(&folder) else {
        return;
    };
    let files: Vec<(String, u64)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let bytes = entry.metadata().ok()?.len();
            Some((name, bytes))
        })
        .collect();
    for doomed in retention(&files) {
        let _ = std::fs::remove_file(folder.join(doomed));
    }
}

/// A browser keeps no records.
#[cfg(target_arch = "wasm32")]
pub(crate) fn keep(_name: &str, _jsonl: &[u8]) {}
