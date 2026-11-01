//! Object identity for instructions which affect their own subject.
//! Damage attribution deliberately continues to use the original source.
use super::{Resolution, source_version};
use crate::{
    event::{Cause, GameEvent},
    state::GameState,
    zone::Zone,
};
use baylee_core::ids::DamageSourceRef;

/// The answer and proof state of a named-card guess.
#[derive(Clone, Copy, Debug, Hash)]
pub struct GuessOutcome {
    /// Whether a card with the chosen name was actually in the hand.
    pub actual: bool,
    /// What the guessing player claimed.
    pub claimed: bool,
    /// Whether the hand was revealed as proof.
    pub revealed: bool,
}

/// Public-zone objects found through earlier instructions of this resolution.
#[derive(Clone, Debug, Default)]
pub struct SubjectContext {
    second_target_players: baylee_core::ids::SeatSet,
    followed: Vec<DamageSourceRef>,
    pub(super) reanimation_links: Vec<super::linked_reanimation::PendingLink>,
    /// Exact future/current battlefield incarnations from the latest search.
    pub(super) found: Vec<DamageSourceRef>,
    pub(super) guess: Option<GuessOutcome>,
    pub(super) named: Option<(crate::object::PrintedFace, baylee_core::ids::NameRef)>,
    pub(super) sacrificed: Option<crate::object::SacrificedLki>,
    circled_colors: baylee_core::color::ColorSet,
    acquisition: Option<u64>,
    /// Original ability source, separate from any successor found by its costs.
    pub(super) source_version: Option<u32>,
    mana_tap_source: Option<DamageSourceRef>,
    event_player: Option<baylee_core::ids::PlayerId>,
    event_identity: Option<(DamageSourceRef, i16)>,
    pub(super) for_each_remaining: Option<Vec<DamageSourceRef>>,
    pub(super) destruction_batches: Vec<super::destruction_receipts::Batch>,
    pub(super) payment_batches: Vec<super::choices::PaymentBatch>,
    checkpoint: Option<(Vec<DamageSourceRef>, u64)>,
}
impl SubjectContext {
    /// Preserve the independent player targets of the second target instance.
    pub(crate) fn with_second_players(mut self, players: baylee_core::ids::SeatSet) -> Self {
        self.second_target_players = players;
        self
    }

    /// Current second-instance player targets, already narrowed for legality.
    pub(crate) fn second_players(&self) -> baylee_core::ids::SeatSet {
        self.second_target_players
    }

    /// A mana ability has no stack object to retain its cost record.
    pub(crate) fn after_cost(paid: &crate::object::PaidRecord) -> Self {
        Self {
            second_target_players: baylee_core::ids::SeatSet::new(),
            followed: paid.source_after_cost.into_iter().collect(),
            reanimation_links: Vec::new(),
            found: Vec::new(),
            guess: None,
            named: None,
            sacrificed: paid.sacrificed_lki,
            circled_colors: baylee_core::color::ColorSet::EMPTY,
            acquisition: None,
            source_version: None,
            mana_tap_source: None,
            event_player: None,
            event_identity: None,
            for_each_remaining: None,
            destruction_batches: Vec::new(),
            payment_batches: Vec::new(),
            checkpoint: None,
        }
    }

    /// Deck-building words belong to the activated ability, even after costs move its source.
    pub(crate) fn with_circled_colors(mut self, colors: baylee_core::color::ColorSet) -> Self {
        self.circled_colors = colors;
        self
    }

    pub(super) fn circled_colors(&self) -> baylee_core::color::ColorSet {
        self.circled_colors
    }

    /// Immediate mana resolutions retain their linked family without a stack object.
    pub(crate) fn with_acquisition(mut self, acquisition: u64) -> Self {
        self.acquisition = Some(acquisition);
        self
    }

    /// Immediate mana abilities have no stack object to retain their source version.
    pub(crate) fn with_source_version(mut self, version: Option<u32>) -> Self {
        self.source_version = version;
        self
    }

    pub(crate) fn with_mana_tap_source(mut self, source: Option<DamageSourceRef>) -> Self {
        self.mana_tap_source = source;
        self
    }
    pub(super) fn mana_tap_source(&self) -> Option<DamageSourceRef> {
        self.mana_tap_source
    }

    pub(crate) fn with_event_player(mut self, player: Option<baylee_core::ids::PlayerId>) -> Self {
        self.event_player = player;
        self
    }
    pub(super) fn set_event_player(&mut self, player: Option<baylee_core::ids::PlayerId>) {
        self.event_player = player;
    }
    pub(super) const fn event_player(&self) -> Option<baylee_core::ids::PlayerId> {
        self.event_player
    }

    /// Preserve the exact event object when an immediate ability has no stack object.
    pub(crate) fn with_event_identity(mut self, identity: Option<(DamageSourceRef, i16)>) -> Self {
        self.event_identity = identity;
        self
    }

    pub(super) fn set_event_identity(&mut self, identity: Option<(DamageSourceRef, i16)>) {
        self.event_identity = identity;
    }

    pub(super) const fn event_identity(&self) -> Option<(DamageSourceRef, i16)> {
        self.event_identity
    }

    pub(crate) fn destruction_progress(&self) -> u64 {
        if self.destruction_batches.is_empty() {
            0
        } else {
            crate::state::structural_fingerprint(&self.destruction_batches)
        }
    }

    pub(crate) fn fingerprint(&self) -> u64 {
        let links = crate::state::structural_fingerprint(&(
            &self.reanimation_links,
            self.second_target_players,
        ));
        let mut h = self
            .sacrificed
            .map_or(0, |lki| crate::state::structural_fingerprint(&lki));
        h = h
            .wrapping_mul(31)
            .wrapping_add(u64::from(self.circled_colors.bits()));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.acquisition));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.source_version));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.mana_tap_source));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.event_player));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(
                &self.for_each_remaining,
            ));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(
                &self.destruction_batches,
            ));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.event_identity));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.payment_batches));
        for r in &self.followed {
            h = h.wrapping_mul(31).wrapping_add(u64::from(r.object.slot()));
            h = h
                .wrapping_mul(31)
                .wrapping_add(u64::from(r.object.generation()));
            h = h.wrapping_mul(31).wrapping_add(u64::from(r.version));
        }
        h = h.wrapping_mul(31).wrapping_add(self.found.len() as u64);
        for r in &self.found {
            h = h.wrapping_mul(31).wrapping_add(u64::from(r.object.slot()));
            h = h
                .wrapping_mul(31)
                .wrapping_add(u64::from(r.object.generation()));
            h = h.wrapping_mul(31).wrapping_add(u64::from(r.version));
        }
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.guess));
        h = h
            .wrapping_mul(31)
            .wrapping_add(crate::state::structural_fingerprint(&self.named));
        // Checkpoints exist only during synchronous resume calls. Every
        // public resume wrapper flushes before returning a held resolution.
        debug_assert!(self.checkpoint.is_none());
        h.wrapping_mul(31).wrapping_add(links)
    }
}

pub(super) fn acquisition(state: &GameState, res: &Resolution) -> u64 {
    res.subject.acquisition.unwrap_or_else(|| {
        state
            .object(res.on_stack)
            .or_else(|| state.object(res.source))
            .map_or(0, crate::object::GameObject::rules_acquisition)
    })
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
        .or_else(|| aura_successor(state, res).filter(|reference| is_current(state, *reference)))
        // A delayed trigger watching its own source die (Golden Guardian's
        // "when this creature dies this turn, return it"): the event object
        // is the source's card, as the graveyard has it now.
        .or_else(|| {
            event(state, res).filter(|reference| {
                reference.object == res.source
                    && is_current(state, *reference)
                    && state
                        .object(reference.object)
                        .is_some_and(|o| o.zone == Zone::Graveyard)
            })
        })
}

/// The Aura's graveyard incarnation its departure trigger recorded
/// (`Rider::SourceAuraSuccessor`, CR 400.7f).
fn aura_successor(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    let version = state
        .object(res.on_stack)?
        .riders
        .iter()
        .find_map(|r| match r {
            crate::object::Rider::SourceAuraSuccessor(v) => Some(*v),
            _ => None,
        })?;
    Some(DamageSourceRef {
        object: res.source,
        version,
    })
}

/// Event incarnation, or its successful public successor in this resolution.
pub(super) fn event(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    let object = res
        .subject
        .event_identity
        .map(|(reference, _)| reference.object)
        .or(res.event_object)?;
    res.subject
        .followed
        .iter()
        .rev()
        .find(|r| r.object == object)
        .copied()
        .or_else(|| {
            super::event_object_identity(state, res)
                .map(|(version, _)| DamageSourceRef { object, version })
        })
        .or_else(|| state.source_identity(object))
}

/// The object an instruction names, retaining the event's incarnation for
/// implicit subjects. An explicit target still uses its resolution snapshot.
pub(super) fn target(
    state: &GameState,
    res: &Resolution,
    object: baylee_core::ids::ObjectId,
) -> Option<DamageSourceRef> {
    res.subject
        .followed
        .iter()
        .rev()
        .find(|r| r.object == object)
        .copied()
        .or_else(|| {
            let stack = state.object(res.on_stack)?;
            if res.event_object != Some(object)
                || (res.targeted
                    && !stack.target_req.is_some_and(|req| {
                        matches!(req.spec, baylee_cards_dsl::TargetSpec::EventObject)
                    }))
            {
                return None;
            }
            // Synthetic untargeted abilities can carry an implicit event
            // subject too. It remains the incarnation that caused the
            // trigger, even when the same card leaves and returns.
            let (version, _) = super::event_object_identity(state, res)?;
            Some(DamageSourceRef { object, version })
        })
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
}

pub(super) fn this(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    if names_targets(state, res) {
        res.targets
            .first()
            .and_then(|&object| target(state, res, object))
    } else {
        source(state, res)
    }
}

/// Whether "this" in the resolving instruction is its object target rather
/// than its source.
///
/// It is the target when the ability said "target" about an object, and
/// that is read off what was announced, which the stack object keeps, and
/// not off what is still legal: an object target that became illegal is
/// not affected (CR 608.2b), and the source does not stand in for it, even
/// while a player chosen beside it is still a legal target. It is also the
/// target, and so nothing, when the ability said "target" and nothing at
/// all was chosen (an "up to one" left empty: Karn's +1). Only a target that
/// is a player and nothing else (Jinxed Choker's "target opponent gains
/// control of this artifact and puts a charge counter on it") leaves "this"
/// the source.
pub(super) fn names_targets(state: &GameState, res: &Resolution) -> bool {
    if !res.targets.is_empty() {
        return true;
    }
    res.targeted
        && (res.target_players.is_empty()
            || state
                .object(res.on_stack)
                .is_some_and(|announced| !announced.targets.is_empty()))
}
/// The creature the resolving trigger is about besides its event object
/// (`Rider::EventSubject`), as it was when the trigger fired.
pub(super) fn event_subject(state: &GameState, res: &Resolution) -> Option<DamageSourceRef> {
    state
        .object(res.on_stack)?
        .riders
        .iter()
        .find_map(|rider| match rider {
            crate::object::Rider::EventSubject(subject) => Some(*subject),
            _ => None,
        })
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
    refs.extend(res.subject.event_identity.map(|(reference, _)| reference));
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
