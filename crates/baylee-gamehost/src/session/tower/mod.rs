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
    adjudicate::verify::<baylee_engine::engine::DetectLoops, super::RegistryLookup>(
        preset,
        None,
        bytes,
        budget::Limits::legacy(),
        deadline,
    )
}

mod adjudicate;
mod budget;
mod metadata;
mod replay;
mod resume;
mod stream;
mod supplies;
pub use budget::ReplayBudget;
use replay::preflight;
pub use resume::{Resumed, resume_native};
use supplies::Sources;

/// Replay the exact native Tower artifact with the repetition detector absent.
/// The preset comes from the authority's frozen package; every Expert answer is
/// recomputed. Streaming input is bounded per line and cumulatively before parsing.
/// A supervising process must additionally enforce memory and wall limits inside
/// individual engine decisions. Exhaustion is technical, never a game outcome.
/// # Errors
/// Tampered/incompatible records, wrong registry/build or technical budget exhaustion.
pub fn verify_native<R: super::SessionRegistry>(
    preset: &GamePreset,
    identity: &baylee_protocol::execution::Identity,
    source: impl std::io::BufRead,
    resources: ReplayBudget,
    deadline: Instant,
) -> Result<Verified, Refused> {
    native_preset::<R>(preset, identity)?;
    adjudicate::verify::<baylee_engine::engine::NoLoopDetection, R>(
        preset,
        Some(identity),
        source,
        resources.limits()?,
        deadline,
    )
}

fn native_preset<R: super::SessionRegistry>(
    preset: &GamePreset,
    identity: &baylee_protocol::execution::Identity,
) -> Result<(), Refused> {
    use baylee_protocol::execution::{Identity, Variant};
    if !R::supports_execution(Variant::Tower)
        || !replay::valid_preset(preset)
        || R::default().validate_rule_pack(preset).is_err()
    {
        return Err(Refused::Invalid);
    }
    let SeatController::Ai(profile) = preset.seats[1].controller else {
        return Err(Refused::Invalid);
    };
    let expected = Identity::native(
        Variant::Tower,
        baylee_build::short(),
        baylee_build::COMMIT,
        u32::from(profile.generation),
    );
    if identity != &expected {
        return Err(Refused::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
