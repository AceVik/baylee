//! Entry replacements which must finish before an object reaches the battlefield.
mod auras;
mod battles;
mod choices;
mod commit;
mod copying;
mod counters;
mod departure;
mod equilibrium;
mod forms;
mod tapping;
pub(crate) use choices::{advance, answer, next_question};
use commit::{finish, sacrifice};
use copying::{answer_copy, copy_question};
pub(crate) use departure::refresh_after_departure;
pub(crate) use equilibrium::after_entry;
pub(crate) use tapping::handles as is_orientation_modifier;

use std::collections::{BTreeSet, VecDeque};
use std::hash::{Hash, Hasher};

use baylee_cards_dsl::{AbilityDef, CopyMod, CounterKind, EnterModifier, Filter};
use baylee_core::ids::{ObjectId, PlayerId};

use crate::{
    choice::{
        ArrangePile, ArrangePlace, ArrangePrompt, ChoicePrompt, Pending, PlayerAction, YesNoPrompt,
    },
    event::{Cause, GameEvent},
    state::{CardLookup, GameState},
    zone::{Zone, ZoneLocation, ZonePosition},
};

#[derive(Clone, Debug, Default)]
pub(crate) struct Entries {
    pending: VecDeque<Attempt>,
    /// Fully decided co-arrivals, retained until the whole counter transaction is ready.
    ready: VecDeque<Attempt>,
    counter_transaction: bool,
    /// Lookup only; the queue is the authoritative ordered pending state.
    pending_objects: BTreeSet<ObjectId>,
    /// Derived queue caches: insertion/departure invalidates the APNAP order.
    order_prepared: bool,
    simple_order: bool,
    /// All participants of the current entry instruction, including arrivals
    /// already committed internally. Built incrementally, never rescanned per token.
    event_objects: BTreeSet<ObjectId>,
    question: Option<Question>,
    committing: Option<ObjectId>,

    putter: Option<PlayerId>,
    after: VecDeque<AfterEntry>,
    chosen_entry: Option<ObjectId>,
}

#[derive(Clone, Debug, Hash)]
struct AfterEntry {
    player: PlayerId,
    remaining: u32,
}

#[derive(Clone, Debug, Hash)]
struct Attempt {
    object: ObjectId,
    version: u32,
    controller: PlayerId,
    putter: PlayerId,
    applied: Vec<(
        baylee_core::ids::DamageSourceRef,
        baylee_cards_dsl::ReplacementRule,
    )>,
    tapped: bool,
    sacrifices_after: u32,
    own_first: bool,
    position: ZonePosition,
    cause: Cause,
    modifiers: &'static [EnterModifier],
    next: usize,
    refusal: Option<Refusal>,
    sacrifices: Option<u32>,
    chosen_number: Option<u8>,
    battle_protector: Option<PlayerId>,
    counters: Vec<(Option<PlayerId>, CounterKind, u16)>,
    counter_plans: Vec<crate::replacement::counter_order::CounterPlacementPlan>,
    counters_prepared: bool,
    counter_caps: Vec<(CounterKind, u16)>,
    next_counter: usize,
    copy: CopyStatus,
    preview_effects: Vec<baylee_core::ids::EffectId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Refusal {
    Graveyard,
    StayInZone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CopyStatus {
    Pending,
    None,
    Handled,
}

#[derive(Clone, Debug, Hash)]
enum Question {
    BattleProtector,
    AuraHost,
    Counters,
    TapLife(u16),
    TapCounters {
        kind: CounterKind,
        amount: u16,
    },
    TapReveal {
        satisfied: bool,
    },
    Number,
    Shape {
        choices: &'static [&'static [CopyMod]],
    },
    Replacement,
    EntryOrder,
    AfterSacrifice,
    Copy {
        index: u32,
        mods: &'static [CopyMod],
        until_eot: bool,
    },
    AcceptSacrifice {
        options: Vec<ObjectId>,
        count: u32,
    },
    Sacrifice {
        options: Vec<ObjectId>,
        remaining: u32,
        chosen: Vec<ObjectId>,
    },
    SacrificeCount {
        options: Vec<ObjectId>,
    },
    Discard,
    DiscardDestination {
        card: ObjectId,
    },
    ExileFromGraveyard {
        count: u32,
    },
    ChooseCounter {
        kinds: &'static [CounterKind],
        remaining: u32,
    },
    PayLifeUpTo,
    GraveyardOrder,
}

impl Hash for Entries {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.pending.hash(h);
        self.ready.hash(h);
        self.counter_transaction.hash(h);
        self.question.hash(h);
        self.committing.hash(h);
        self.after.hash(h);
        self.chosen_entry.hash(h);
        // Event participants affect replacement choices only while an arrival is pending.
        if !self.pending.is_empty() {
            self.event_objects.hash(h);
        }
    }
}

impl Entries {
    pub(crate) fn begin_event(&mut self, _seq: u64, putter: Option<PlayerId>) {
        if self.pending.is_empty() && self.ready.is_empty() && self.question.is_none() {
            self.event_objects.clear();
            self.counter_transaction = false;
            self.putter = putter;
        }
    }
    pub(crate) fn copying(&self) -> Option<(ObjectId, &'static [CopyMod])> {
        let Question::Copy { mods, .. } = self.question.as_ref()? else {
            return None;
        };
        Some((self.pending.front()?.object, mods))
    }
    pub(crate) fn has_after(&self) -> bool {
        !self.after.is_empty()
    }
    pub(crate) fn set_putter(&mut self, player: PlayerId) {
        self.putter = Some(player);
    }
    /// The player putting permanents onto the battlefield in the current
    /// event: the controller of the resolving spell or ability, the player
    /// playing a land. `None` where no event named one.
    pub(crate) const fn putter(&self) -> Option<PlayerId> {
        self.putter
    }
    pub(crate) fn waiting(&self) -> bool {
        self.question.is_some()
    }
    pub(crate) fn contains(&self, id: ObjectId) -> bool {
        self.pending_objects.contains(&id)
    }
}

fn modifiers(state: &GameState, id: ObjectId) -> &'static [EnterModifier] {
    let Some(object) = state.object(id) else {
        return &[];
    };
    let Some(face) = object.printed_face() else {
        return &[];
    };
    state
        .bases
        .card(face.card())
        .and_then(|card| card.faces.get(usize::from(face.face())))
        .map_or(&[], |face| face.enter_modifiers)
}

/// Printed entry text after projecting the prospective permanent's ability loss.
fn projected_modifiers(state: &GameState, id: ObjectId) -> &'static [EnterModifier] {
    let Some(object) = state.object(id) else {
        return &[];
    };
    let mut prospective = object.clone();
    prospective.zone = Zone::Battlefield;
    prospective.version = prospective.version.wrapping_add(1);
    let plan = crate::layers::LayerPlan::before_entries(&state.effects, unavailable(state));
    let projection = crate::layers::recompute_with(state, &prospective, &plan);
    if projection.characteristics.rules_text_lost
        || projection.characteristics.abilities_lost.is_some()
    {
        &[]
    } else {
        modifiers(state, id)
    }
}

fn copy_clause(
    state: &GameState,
    id: ObjectId,
) -> Option<(u32, baylee_cards_dsl::TargetSpec, &'static [CopyMod], bool)> {
    state
        .object(id)?
        .printed_abilities(state.bases.as_ref())
        .iter()
        .enumerate()
        .find_map(|(index, ability)| match ability {
            AbilityDef::CopyOnEnter { target, mods } => Some((
                u32::try_from(index).expect("ability index fits"),
                *target,
                *mods,
                false,
            )),
            AbilityDef::CopyOnEnterUntilEot { target, mods } => Some((
                u32::try_from(index).expect("ability index fits"),
                *target,
                *mods,
                true,
            )),
            _ => None,
        })
}

fn needs_before_entry(modifier: &EnterModifier) -> bool {
    tapping::handles(modifier)
        || matches!(
            modifier,
            EnterModifier::ChooseNumber { .. }
                | EnterModifier::ChooseShape(_)
                | EnterModifier::SacrificeOrGraveyard { .. }
                | EnterModifier::DiscardOrGraveyard(_)
                | EnterModifier::SacrificeForPowerToughness(_)
                | EnterModifier::ExileFromGraveyardXOrGraveyard(_)
                | EnterModifier::ChooseCounterPerEntryCount(_)
                | EnterModifier::PayLifeUpTo(_)
                | EnterModifier::SacrificeOthers(_)
        )
}

pub(crate) fn set_chosen_number(state: &mut GameState, id: ObjectId, number: u8, acquisition: u64) {
    if let Some(object) = state.object_mut(id) {
        object
            .riders
            .retain(|rider| !matches!(rider, crate::object::Rider::ChosenNumber { acquisition: previous, .. } if *previous == acquisition));
        object.riders.push(crate::object::Rider::ChosenNumber {
            acquisition,
            number,
        });
    }
    state.invalidate_projections();
}

/// Returns true when the move is deferred. Setup deliberately installs its fixtures directly.
pub(crate) fn defer(
    state: &mut GameState,
    id: ObjectId,
    position: ZonePosition,
    cause: Cause,
) -> bool {
    if state.entries.committing == Some(id) {
        state.entries.committing = None;
        return false;
    }
    if cause == Cause::Setup {
        return false;
    }
    if state.entries.contains(id) {
        return true;
    }
    state.entries.event_objects.insert(id);
    let mut modifiers = modifiers(state, id);
    let copy = copy_clause(state, id);
    let external = equilibrium::exists(state);
    let aura = auras::needs_choice(state, id);
    let counter_entry = counters::needs_entry(state, id) || battles::needs_entry(state, id);
    state.entries.counter_transaction |= counter_entry;
    if !aura
        && !external
        && !counter_entry
        && copy.is_none()
        && !modifiers.iter().any(needs_before_entry)
    {
        return false;
    }
    let Some(object) = state.object(id) else {
        return false;
    };
    let mut prospective = object.clone();
    prospective.zone = Zone::Battlefield;
    prospective.version = prospective.version.wrapping_add(1);
    let plan = crate::layers::LayerPlan::before_entries(&state.effects, unavailable(state));
    let projected = crate::layers::recompute_with(state, &prospective, &plan);
    let lost = projected.characteristics.rules_text_lost
        || projected.characteristics.abilities_lost.is_some();
    if lost {
        modifiers = &[];
    }
    if (lost || copy.is_none())
        && !modifiers.iter().any(needs_before_entry)
        && !external
        && !aura
        && !counter_entry
    {
        return false;
    }
    let attempt = Attempt {
        object: id,
        version: object.version,
        controller: projected.controller,
        putter: state.entries.putter.unwrap_or(object.controller),
        applied: Vec::new(),
        tapped: object.status.contains(crate::object::Status::TAPPED),
        sacrifices_after: 0,
        own_first: false,
        position,
        cause,
        modifiers,
        next: 0,
        refusal: None,
        sacrifices: None,
        chosen_number: None,
        battle_protector: None,
        counters: Vec::new(),
        counter_plans: Vec::new(),
        counters_prepared: false,
        counter_caps: Vec::new(),
        next_counter: 0,
        copy: if lost {
            CopyStatus::None
        } else {
            CopyStatus::Pending
        },
        preview_effects: Vec::new(),
    };
    state.entries.pending_objects.insert(id);
    state.entries.order_prepared = false;
    state.entries.pending.push_back(attempt);
    true
}

/// Identity a successful battlefield move will create, even while its entry
/// replacements are waiting. Callers must check the zone and incarnation again
/// before acting: an entry can be denied or the permanent can leave meanwhile.
pub(crate) fn battlefield_reference(
    state: &GameState,
    id: ObjectId,
) -> Option<baylee_core::ids::DamageSourceRef> {
    let object = state.object(id)?;
    let version = if object.zone == Zone::Battlefield {
        object.version
    } else {
        state
            .entries
            .pending
            .iter()
            .chain(&state.entries.ready)
            .find(|entry| entry.object == id && entry.version == object.version)?
            .version
            .wrapping_add(1)
    };
    Some(baylee_core::ids::DamageSourceRef {
        object: id,
        version,
    })
}

/// A resolving search/reanimation can attach its entry counters to a deferred move.
pub(crate) fn put_counters(
    state: &mut GameState,
    by: PlayerId,
    id: ObjectId,
    kind: CounterKind,
    n: u16,
) {
    if n == 0 {
        return;
    }
    if let Some(entry) = state
        .entries
        .pending
        .iter_mut()
        .find(|entry| entry.object == id)
    {
        entry.counters.push((Some(by), kind, n));
    } else if state
        .object(id)
        .is_some_and(|o| o.zone == Zone::Battlefield)
    {
        crate::replacement::put_counters_by(state, by, id, kind, n);
    }
}

fn unavailable(state: &GameState) -> &BTreeSet<ObjectId> {
    &state.entries.event_objects
}

fn options(state: &GameState, entry: &Attempt, filter: &Filter, hand: bool) -> Vec<ObjectId> {
    let unavailable = unavailable(state);
    let ids: Vec<_> = if hand {
        state
            .zones
            .list(ZoneLocation::Hand(entry.controller))
            .clone()
    } else {
        state.battlefield_seen().collect()
    };
    ids.into_iter()
        .filter(|id| *id != entry.object && !unavailable.contains(id))
        .filter(|id| {
            state.object(*id).is_some_and(|object| {
                (hand || object.controller == entry.controller)
                    && crate::eval::matches(filter, state, object, entry.controller, entry.object)
            })
        })
        .collect()
}

/// Entry counters without a named placing player belong to the entering
/// object's controller (CR 122.6a), after entry control replacements.
pub(crate) fn put_entry_counters(state: &mut GameState, id: ObjectId, kind: CounterKind, n: u16) {
    if n == 0 {
        return;
    }
    if let Some(entry) = state
        .entries
        .pending
        .iter_mut()
        .find(|entry| entry.object == id)
    {
        entry.counters.push((None, kind, n));
    } else if let Some(controller) = state
        .object(id)
        .filter(|object| object.zone == Zone::Battlefield)
        .map(|object| object.controller)
    {
        crate::replacement::put_counters_by(state, controller, id, kind, n);
    }
}
