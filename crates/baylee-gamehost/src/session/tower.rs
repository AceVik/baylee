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

fn preflight(preset: &GamePreset, bytes: &[u8], deadline: Instant) -> Result<(), Refused> {
    if Instant::now() >= deadline {
        return Err(Refused::Budget);
    }
    if bytes.is_empty() || bytes.len() > MAX_RECORD_BYTES {
        return Err(Refused::Budget);
    }
    if !bytes.ends_with(b"\n")
        || preset.format != FormatId::Custom
        || preset.validate().is_err()
        || !super::challenge::valid(preset)
        || preset.house_rules.challenge_policy_seed.is_none()
        || preset.seats[0].controller != SeatController::Open
        || preset.seats[1].controller != SeatController::Ai(AIProfile::TOWER_EXPERT)
        || preset
            .seats
            .iter()
            .any(|seat| seat.capabilities != baylee_core::preset::SeatCapabilities::default())
    {
        return Err(Refused::Invalid);
    }
    for (n, line) in bytes[..bytes.len() - 1]
        .split(|byte| *byte == b'\n')
        .enumerate()
    {
        if n >= MAX_RECORD_LINES || line.len() > MAX_LINE_BYTES {
            return Err(Refused::Budget);
        }
        if line.is_empty() {
            return Err(Refused::Invalid);
        }
    }
    Ok(())
}

fn budget(session: &Session, deadline: Instant) -> Result<(), Refused> {
    if Instant::now() >= deadline
        || session.engine.journal().len() > MAX_EVENTS
        || session.engine.state().turn.number > 500
    {
        return Err(Refused::Budget);
    }
    Ok(())
}

fn apply(
    session: &mut Session,
    checker: &mut Checker,
    line: Line,
    next: u64,
    ended: &mut bool,
) -> Result<u64, Refused> {
    match line {
        Line::Input {
            n,
            at,
            seat,
            by,
            action,
            hash,
            seen,
        } if n == next => {
            if matches!(session.engine.pending(), Pending::GameOver(_)) {
                return Err(Refused::Invalid);
            }
            session.tell_time(at);
            if !session.replayed(PlayerId::new(seat), by, action) {
                return Err(Refused::Invalid);
            }
            checker
                .check(&session.engine, Some(n), hash, seen.as_deref())
                .map_err(|_| Refused::Invalid)?;
            Ok(at)
        }
        // A seat's self-description never changes the policy or the result.
        Line::DeclaredMind { n, at, seat: 0, .. } if n == next => Ok(at),
        Line::End {
            n,
            at,
            winners,
            reason,
            turn,
        } if n == next => {
            let Pending::GameOver(result) = session.engine.pending() else {
                return Err(Refused::Invalid);
            };
            let actual: Vec<u8> = session
                .winning_seats(*result)
                .iter()
                .map(|seat| seat.get())
                .collect();
            if winners != actual
                || reason != format!("{:?}", result.reason)
                || turn != Some(session.engine.state().turn.number)
                || !(1..=500).contains(&session.engine.state().turn.number)
            {
                return Err(Refused::Invalid);
            }
            *ended = true;
            Ok(at)
        }
        _ => Err(Refused::Invalid),
    }
}

#[derive(Clone, Copy)]
struct Source {
    id: ObjectId,
    version: u32,
    charges: u16,
}
struct Sources([[Option<Source>; 2]; 2]);

impl Sources {
    fn of(session: &Session) -> Self {
        let state = session.engine.state();
        Self(std::array::from_fn(|seat| {
            std::array::from_fn(|kind| {
                let supply = if kind == 0 {
                    TowerSupply::HealingPotion
                } else {
                    TowerSupply::ManaPotion
                };
                let definition = baylee_cards::tokens::tower::definition(supply);
                state
                    .zones
                    .list(ZoneLocation::Command(PlayerId::new(
                        u8::try_from(seat).expect("two seats"),
                    )))
                    .iter()
                    .find_map(|id| {
                        let object = state.object(*id)?;
                        object
                            .token
                            .filter(|token| std::ptr::eq(*token, definition))
                            .map(|_| Source {
                                id: *id,
                                version: object.version,
                                charges: object.counters.get(CounterKind::Charge),
                            })
                    })
            })
        }))
    }

    fn consumed(&self, session: &Session) -> Result<[[u8; 2]; 2], Refused> {
        let mut paid = [[0_u16; 2]; 2];
        for entry in session.engine.journal().entries() {
            if let GameEvent::CounterChanged {
                object,
                version,
                kind: CounterKind::Charge,
                old,
                new,
                ..
            } = entry.event
            {
                for (seat, sources) in self.0.iter().enumerate() {
                    for (kind, source) in sources.iter().enumerate() {
                        if let Some(source) = source
                            && source.id == object
                            && source.version == version
                        {
                            // Original command-zone charges can only be spent, never replenished.
                            if old != source.charges - paid[seat][kind]
                                || old.checked_sub(1) != Some(new)
                            {
                                return Err(Refused::Invalid);
                            }
                            paid[seat][kind] += 1;
                        }
                    }
                }
            }
        }
        let mut consumed = [[0; 2]; 2];
        for (out, paid) in consumed
            .iter_mut()
            .flatten()
            .zip(paid.into_iter().flatten())
        {
            *out = u8::try_from(paid).map_err(|_| Refused::Invalid)?;
        }
        Ok(consumed)
    }
}

#[cfg(test)]
mod tests;
