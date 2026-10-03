//! Object identity for instructions which affect their own subject.
//! Damage attribution deliberately continues to use the original source.
use super::{Resolution, source_version};
use crate::{
    event::{Cause, GameEvent},
    state::GameState,
    zone::Zone,
};
use baylee_core::ids::DamageSourceRef;

/// Public-zone objects found through earlier instructions of this resolution.
#[derive(Clone, Debug, Default)]
pub struct SubjectContext {
    followed: Vec<DamageSourceRef>,
    checkpoint: Option<(Vec<DamageSourceRef>, u64)>,
}
impl SubjectContext {
    /// A mana ability has no stack object to retain its cost record.
    pub(crate) fn after_cost(source: Option<DamageSourceRef>) -> Self {
        Self {
            followed: source.into_iter().collect(),
            checkpoint: None,
        }
    }

    pub(crate) fn fingerprint(&self) -> u64 {
        let mut h = 0_u64;
        for r in &self.followed {
            h = h.wrapping_mul(31).wrapping_add(u64::from(r.object.slot()));
            h = h
                .wrapping_mul(31)
                .wrapping_add(u64::from(r.object.generation()));
            h = h.wrapping_mul(31).wrapping_add(u64::from(r.version));
        }
        // Checkpoints exist only during synchronous resume calls. Every
        // public resume wrapper flushes before returning a held resolution.
        debug_assert!(self.checkpoint.is_none());
        h
    }
}

pub(super) fn source(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    res.subject
        .followed
        .iter()
        .rev()
        .find(|r| r.object == res.source)
        .copied()
        .or_else(|| paid_source(state, res))
        .or_else(|| {
            source_version(state, res).map(|version| DamageSourceRef {
                object: res.source,
                version,
            })
        })
        .or_else(|| state.source_identity(res.source))
}

// The payment records a successful public-zone successor, never an inferred
// relationship between whichever versions happen to exist now.
fn paid_source(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    state.object(res.on_stack)?.paid.as_ref()?.source_after_cost
}

/// Moving a departed source can find the precise public destination of its
/// own zone-change trigger (CR 400.7e). Characteristic changes and damage do
/// not use this exception merely because the event context was retained.
pub(super) fn moving_source(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    source(state, res)
        .filter(|reference| is_current(state, *reference))
        .or_else(|| {
            state
                .own_departure_successor(res.on_stack)
                .filter(|reference| reference.object == res.source && is_current(state, *reference))
        })
}

pub(super) fn this(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    if let Some(&object) = res.targets.first() {
        res.subject
            .followed
            .iter()
            .rev()
            .find(|r| r.object == object)
            .copied()
            .or_else(|| {
                res.target_lki
                    .as_ref()?
                    .iter()
                    .find(|r| r.id == object)
                    .map(|r| DamageSourceRef {
                        object,
                        version: r.version,
                    })
            })
            .or_else(|| state.source_identity(object))
    } else if res.targeted {
        None
    } else {
        source(state, res)
    }
}
pub(super) fn is_current(state: &GameState, reference: DamageSourceRef) -> bool {
    state.object(reference.object).is_some_and(|o| {
        o.version == reference.version && !o.status.contains(crate::object::Status::PHASED_OUT)
    })
}
pub(super) fn on_battlefield(state: &GameState, reference: DamageSourceRef) -> bool {
    state.object(reference.object).is_some_and(|o| {
        o.version == reference.version
            && o.zone == Zone::Battlefield
            && !o.status.contains(crate::object::Status::PHASED_OUT)
    })
}

pub(super) fn before(state: &GameState, res: &Resolution) -> (Vec<DamageSourceRef>, u64) {
    let mut refs: Vec<_> = source(state, res).into_iter().collect();
    if let Some(event) = res.event_object {
        let version = super::event_object_identity(state, res).map(|(v, _)| v);
        if let Some(version) = version {
            refs.push(DamageSourceRef {
                object: event,
                version,
            });
        }
    }
    if let Some(targets) = &res.target_lki {
        refs.extend(targets.iter().map(|t| DamageSourceRef {
            object: t.id,
            version: t.version,
        }));
    }
    refs.extend(res.subject.followed.iter().copied());
    refs.retain(|r| state.source_identity(r.object) == Some(*r));
    refs.sort_unstable();
    refs.dedup();
    (refs, state.journal.last_seq())
}
/// A successor reached solely through actual public-zone moves of this operation.
pub(crate) fn public_successor(
    state: &GameState,
    original: DamageSourceRef,
    since: u64,
    cause: Cause,
) -> Option<DamageSourceRef> {
    let current = state.source_identity(original.object)?;
    if current == original {
        return None;
    }
    let mut moved = false;
    for entry in state.journal.entries().iter().filter(|e| e.seq > since) {
        if let GameEvent::ZoneChanged {
            object,
            to,
            cause: actual,
            ..
        } = entry.event
            && object == original.object
        {
            if to.is_hidden_by_default() || actual != cause {
                return None;
            }
            moved = true;
        }
    }
    moved.then_some(current)
}
pub(super) fn after(state: &GameState, res: &mut Resolution, before: &(Vec<DamageSourceRef>, u64)) {
    for original in &before.0 {
        let Some(current) = public_successor(state, *original, before.1, Cause::Effect) else {
            continue;
        };
        res.subject.followed.retain(|r| r.object != current.object);
        res.subject.followed.push(current);
    }
}
pub(super) fn begin_resume(state: &GameState, res: &mut Resolution) {
    flush(state, res);
    res.subject.checkpoint = Some(before(state, res));
}
pub(super) fn flush(state: &GameState, res: &mut Resolution) {
    if let Some(before) = res.subject.checkpoint.take() {
        after(state, res, &before);
    }
}
