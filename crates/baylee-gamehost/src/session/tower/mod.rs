//! Strict terminal replay for a reserved Tower duel, including the fixed Expert policy.
//! The caller authenticates execution capacity and supplies its own stored preset.
use super::Session;
use crate::record::{self, Checker, Line, RECORD_VERSION};
use baylee_core::{
    ids::{ObjectId, PlayerId},
    preset::{AIProfile, FormatId, GamePreset, SeatController, TowerSupply},
};
use baylee_engine::{
    choice::Pending, event::GameEvent, object::CounterKind, win::GameResult, zone::ZoneLocation,
};
use sha2::{Digest, Sha256};
use std::time::Instant;

/// Maximum record bytes accepted before JSON parsing or engine construction.
pub const MAX_RECORD_BYTES: usize = 16 * 1024 * 1024;
/// Maximum lines, including metadata, independently of encoded byte length.
pub const MAX_RECORD_LINES: usize = 50_000;
const MAX_LINE_BYTES: usize = 128 * 1024;
const MAX_EVENTS: usize = 500_000;

/// Refusal categories contain no deck, token, record text or identifying payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// Malformed, incomplete, incompatible, reordered or rule-inconsistent record.
    Invalid,
    /// Bounded work or the worker's deadline was exhausted; no result was awarded.
    Budget,
}

/// Evidence recomputed from the engine; never deserialize this from an executor's claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verified {
    /// Engine-derived terminal result, independent of the submitted winner label.
    pub result: GameResult,
    /// Engine-derived terminal turn.
    pub turn: u32,
    /// Actual charge payments per seat, healing then mana; elimination is not payment.
    pub consumed: [[u8; 2]; 2],
    /// Digest of these exact accepted record bytes, for durable submission deduplication.
    pub record_sha256: [u8; 32],
}

/// Verify the exact reserved game, with every AI decision recomputed from its view.
/// A worker additionally bounds process memory/concurrency and enforces its wall deadline:
/// this cooperative deadline is checked between engine decisions, not inside a decision.
/// # Errors
/// [`Refused`] on tampering, missing termination, incompatible material or work exhaustion.
pub fn verify(preset: &GamePreset, bytes: &[u8], deadline: Instant) -> Result<Verified, Refused> {
    preflight(preset, bytes, deadline)?;
    let mut lines = bytes[..bytes.len() - 1].split(|byte| *byte == b'\n');
    let header: Line = serde_json::from_slice(lines.next().ok_or(Refused::Invalid)?)
        .map_err(|_| Refused::Invalid)?;
    let Line::Header {
        record: RECORD_VERSION,
        build,
        preset: recorded,
        hash,
        seen,
        ..
    } = header
    else {
        return Err(Refused::Invalid);
    };
    if build != record::this_build() || recorded.as_ref() != preset {
        return Err(Refused::Invalid);
    }
    let mut session = Session::new(preset).ok_or(Refused::Invalid)?;
    let supplies = Sources::of(&session);
    let mut checker = Checker::for_build(&build);
    checker
        .check(&session.engine, None, hash, seen.as_deref())
        .map_err(|_| Refused::Invalid)?;
    let mut ended = false;
    let mut previous_time = 0;
    for (index, raw) in lines.enumerate() {
        if ended {
            return Err(Refused::Invalid);
        }
        budget(&session, deadline)?;
        let line: Line = serde_json::from_slice(raw).map_err(|_| Refused::Invalid)?;
        let n = u64::try_from(index).map_err(|_| Refused::Budget)?;
        let at = apply(&mut session, &mut checker, line, n, &mut ended)?;
        if at < previous_time {
            return Err(Refused::Invalid);
        }
        previous_time = at;
    }
    budget(&session, deadline)?;
    let Pending::GameOver(result) = session.engine.pending() else {
        return Err(Refused::Invalid);
    };
    if !ended {
        return Err(Refused::Invalid);
    }
    Ok(Verified {
        result: *result,
        turn: session.engine.state().turn.number,
        consumed: supplies.consumed(&session)?,
        record_sha256: Sha256::digest(bytes).into(),
    })
}

mod replay;
mod supplies;
use replay::{apply, budget, preflight};
use supplies::Sources;

#[cfg(test)]
mod tests;
