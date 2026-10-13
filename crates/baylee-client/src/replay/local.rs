//! A kept record played again here (§3.1): the projection a local replay
//! runs on its worker, and the Replays list's rows for the kept records.

use std::sync::atomic::{AtomicBool, AtomicU8};

use baylee_client_core::replay::{Header, Perspective, Tape, Verify};

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
    Err("Replaying a game against the house needs baylee_gamehost::replay (lane A).".to_string())
}
