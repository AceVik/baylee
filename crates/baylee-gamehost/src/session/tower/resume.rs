//! Recover only exact native evidence, with the fixed opponent's memory intact.
use super::{GamePreset, Refused, ReplayBudget, Session, adjudicate, native_preset};
use crate::{record::Recorder, session::SessionRegistry};
use baylee_engine::engine::NoLoopDetection;
use baylee_protocol::execution::Identity;
use std::{io::BufRead, time::Instant};

/// Reconstructed game, not yet exposed to any socket. Contains private state.
pub struct Resumed<R: SessionRegistry> {
    session: Session<NoLoopDetection, R>,
    at: u64,
}

impl<R: SessionRegistry> Resumed<R> {
    /// Minimum host timestamp for subsequent record entries.
    #[must_use]
    pub const fn at(&self) -> u64 {
        self.at
    }

    /// Restore presentation without reseeding the already replayed Expert agent.
    /// The authority supplies current permitted display names, never record data.
    #[must_use]
    pub fn into_session(mut self, game: String, names: Vec<String>) -> Session<NoLoopDetection, R> {
        self.session.game_id = game;
        self.session.names = names;
        self.session
    }
}

/// Rebuild a complete committed prefix and continue recording without a new header.
/// Every recorded Expert decision is recomputed, restoring its RNG and memory.
/// The caller checks the authority's chunk commitment and supervises process time
/// and memory; like adjudication, the cooperative deadline cannot stop a rules call.
/// # Errors
/// Wrong artifact, altered/reordered/truncated evidence or exhausted replay budget.
pub fn resume_native<R: SessionRegistry>(
    preset: &GamePreset,
    identity: &Identity,
    source: impl BufRead,
    resources: ReplayBudget,
    deadline: Instant,
) -> Result<Resumed<R>, Refused> {
    native_preset::<R>(preset, identity)?;
    let mut rebuilt = adjudicate::rebuild::<NoLoopDetection, R>(
        preset,
        Some(identity),
        source,
        resources.limits()?,
        deadline,
    )?;
    rebuilt.session.record = Some(Recorder::resumed(rebuilt.next, rebuilt.ended));
    rebuilt.session.tell_time(rebuilt.at);
    Ok(Resumed {
        session: rebuilt.session,
        at: rebuilt.at,
    })
}
