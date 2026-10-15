//! Triggered abilities: collection and APNAP ordering.
//!
//! After every mutation batch the engine scans new journal entries and
//! matches them against the triggered abilities of permanents and cards
//! whose abilities function in other zones. Matches go on the stack in APNAP
//! order. Same-controller ordering currently follows timestamp; the
//! player-facing ordering choice is M2 (documented in engine-internals).

mod activations;
pub(crate) mod bird_entry;
mod discarded;
mod event_groups;
mod radiation;
mod return_keywords;
mod sagas;
mod speed;

pub(crate) use return_keywords::return_keyword_counts;

use crate::eval;
use crate::event::GameEvent;
use crate::state::{CardLookup, GameState};
use crate::text_changes::{RuleContext, TextChangeMap};
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{AbilityDef, Condition, PlayerRel, Trigger, TriggerZone};
use baylee_core::ids::{ObjectId, PlayerId};

/// Mana types actually produced by one activation, including colorless.
#[derive(Clone, Copy, Debug)]
pub struct EventMana {
    /// The player who activated the mana ability.
    pub player: PlayerId,
    /// One bit per `ManaColor`, independent of amount and spending restrictions.
    pub types: u8,
}

impl EventMana {
    /// Compact nonzero key for a captured production in the engine snapshot.
    pub(crate) fn key(self) -> u64 {
        1 + u64::from(self.player.get()) + (u64::from(self.types) << 8)
    }
}

/// A triggered ability waiting to go on the stack.
#[derive(Clone, Debug)]
pub struct PendingTrigger {
    /// The object whose ability triggered.
    pub source: ObjectId,
    /// Event-time source incarnation for linked-counter effects.
    pub counter_source_version: Option<u32>,
    /// Event object incarnation and power, retained for damage it deals later.
    pub event_object_identity: Option<(u32, i16)>,
    /// Exact activated stack ability captured separately from its permanent source.
    pub event_ability: Option<baylee_core::ids::DamageSourceRef>,
    /// Immutable exact-object group for one delayed trigger. Shared on
    /// checkpoints; ordinary triggers allocate nothing and carry one pointer.
    pub related_objects: Option<std::sync::Arc<Vec<baylee_core::ids::DamageSourceRef>>>,
    /// The creature a pair trigger about an attached creature was about
    /// (`Trigger::AttachedBlocksOrBecomesBlockedBy`), as it was then; it
    /// rides on the stack object (`Rider::EventSubject`) and on to the
    /// delayed triggers it makes (`Effect::AtNextEndStepAboutSubject`).
    pub event_subject: Option<baylee_core::ids::DamageSourceRef>,
    /// Source incarnation at the triggering event, including battlefield LKI.
    pub source_version: Option<u32>,
    /// Index into the source card's abilities.
    pub ability_index: u32,
    /// Event-time rules text. Captured even while the source is on the
    /// battlefield: a legend choice or lethal SBA can remove a copy before
    /// this trigger is stacked. Synthetic keyword triggers carry their own
    /// effects instead and leave this `None`.
    pub abilities: Option<crate::object::AbilityList>,
    /// Frozen words of this trigger, independent of later source text changes.
    pub text: TextChangeMap,
    /// Controller of the trigger.
    pub controller: PlayerId,
    /// Timestamp of the source (stable same-controller ordering).
    pub timestamp: u64,
    /// The object the triggering event was about (if any).
    pub event_object: Option<ObjectId>,
    /// The event permanent's mana value before leaving the battlefield.
    pub event_mana_value: Option<u32>,
    /// Controller and toughness captured before the event permanent left play.
    pub event_departure: Option<(PlayerId, i16)>,
    /// Actual production captured for a triggered mana ability.
    pub event_mana: Option<EventMana>,
    /// The amount captured by this trigger: damage dealt or a delayed scry count.
    /// A damage event's recipient is [`Self::event_player`].
    pub event_amount: Option<u32>,
    /// The player the triggering event was about ([`event_player_of`]):
    /// "that player" and "they" of a trigger on a player's event
    /// (`PlayerRel::EventPlayer`). For a damage event it is the player dealt
    /// it.
    pub event_player: Option<PlayerId>,
    /// What an untargeted synthetic trigger puts first among its targets,
    /// which is what its `Filter::This` and its "target" words then name
    /// (`resolve::this_object`).
    ///
    /// The event object for ward (the spell it counters) and the monarch's
    /// takeover (the creature whose controller takes the crown); the
    /// permanent itself for prowess, undying and persist. `None` for a
    /// granted triggered ability: the object that has the ability is its
    /// source (CR 113.7), and a gained ability's word for itself names the
    /// object that gained it (CR 201.5b), which is what `this_object` reads
    /// when there is no target. This used to be `event_object` for all of
    /// them, and Great Hall of the Biblioplex's granted "this creature gets
    /// +1/+0" pumped the instant that had triggered it. The event object is
    /// still carried, for `TargetSpec::EventObject`.
    pub implicit_target: Option<ObjectId>,
    /// The effects of a trigger no card prints. Three things produce one:
    /// an engine-level keyword (prowess, ward), a granted triggered ability,
    /// and a reflexive triggered ability an effect created (CR 603.12,
    /// `resolve::reflexive`).
    pub synthetic_effects: Option<&'static [baylee_cards_dsl::Effect]>,
    /// Target spec for synthetic triggers that need a target choice
    /// (granted triggered abilities, class levels, a reflexive "target").
    /// It is written onto the stack object as its `target_req`, which is
    /// what CR 608.2b re-checks at resolution.
    pub synthetic_target: Option<baylee_cards_dsl::TargetSpec>,
    /// Fires at most once each turn (marked by the engine after stacking).
    pub once_per_turn: bool,
    /// The mode a modal trigger's controller picked, once they have.
    ///
    /// `None` on every trigger as it is collected, including a modal one:
    /// CR 603.3c has the controller announce the mode *as the ability is
    /// put on the stack*, which is later than this. It is written back onto
    /// the queued trigger when the question is answered, and the entry then
    /// walks the same path as any other trigger — the mode's own
    /// `TargetReq` is read through it, and the shared tail records
    /// `once_per_turn` and stacks it.
    pub chosen_mode: Option<u8>,
}

fn object_text(state: &GameState, object: &crate::object::GameObject) -> TextChangeMap {
    state.text_changes.get(baylee_core::ids::DamageSourceRef {
        object: object.id,
        version: object.version,
    })
}

fn ability_context(
    state: &GameState,
    object: &crate::object::GameObject,
    list: &crate::object::AbilityList,
    index: usize,
) -> RuleContext {
    RuleContext {
        source: object.id,
        text: list.base_text(index).then(object_text(state, object)),
        reference: None,
    }
}

fn condition_matches(
    state: &GameState,
    condition: Option<Condition>,
    you: PlayerId,
    context: RuleContext,
    entry: Option<&crate::event::JournalEntry>,
) -> bool {
    let Some(condition) = condition else {
        return true;
    };
    if !eval::condition_reads_event(condition) {
        return eval::condition_holds_with_context(state, you, context, condition);
    }
    // The event's object as the event saw it: the permanent that moved, or
    // the creature a blocker blocked (Wall of Caltrops' "that creature").
    let event = entry.and_then(|entry| {
        let object = match entry.event {
            GameEvent::ZoneChanged { object, .. } => object,
            GameEvent::BecameBlocker { attacker, .. } => attacker,
            _ => return None,
        };
        state
            .event_object_identity(object, entry.seq)
            .map(|(version, _)| baylee_core::ids::DamageSourceRef { object, version })
    });
    eval::condition_with_event(state, you, context, condition, event)
}

fn freeze_trigger(state: &GameState, trigger: &mut PendingTrigger) {
    if let Some(list) = &mut trigger.abilities {
        let live = trigger
            .source_version
            .map_or(TextChangeMap::IDENTITY, |version| {
                state.text_changes.get(baylee_core::ids::DamageSourceRef {
                    object: trigger.source,
                    version,
                })
            });
        *list = list.clone().with_base_text(live);
        trigger.text = list.base_text(trigger.ability_index as usize);
    }
}

/// What every triggered ability says about *whether* it fires, whatever
/// shape it is written in.
///
/// `AbilityDef::ModalTriggered` is a triggered ability — the modes are what
/// it *does*, not whether it fires — and reading only `Triggered` here is
/// what made five cards in the pool resolve to nothing at all (entry 34 in
/// `docs/observed-faults.md`). Both collection loops ask through this
/// function so the pair cannot drift again, and a struct rather than a
/// tuple so that the next thing a trigger carries is one field here and no
/// churn at the three places that read it.
struct Firing {
    /// The event it listens for.
    trigger: &'static Trigger,
    /// Zone in which the printed ability functions.
    zone: TriggerZone,
    /// Whether it fires at most once each turn.
    once_per_turn: bool,
    /// The intervening-`if` clause, if the card prints one (CR 603.4).
    condition: Option<Condition>,
}

const fn triggered_parts(ability: &'static AbilityDef) -> Option<Firing> {
    match ability {
        AbilityDef::Triggered {
            trigger,
            zone,
            once_per_turn,
            condition,
            ..
        }
        | AbilityDef::ModalTriggered {
            trigger,
            zone,
            once_per_turn,
            condition,
            ..
        } => Some(Firing {
            trigger,
            zone: *zone,
            once_per_turn: *once_per_turn,
            condition: *condition,
        }),
        _ => None,
    }
}

/// Matches new journal entries (from `from_seq` onward) against all
/// triggered abilities on the battlefield.
#[must_use]
pub fn collect(state: &GameState, lookup: &impl CardLookup, from_seq: u64) -> Vec<PendingTrigger> {
    let events = &state.journal.entries()[from_seq as usize..];
    if events.is_empty() {
        return Vec::new();
    }
    let mut triggers = Vec::new();
    // A phased-out permanent sees nothing happen (CR 702.26b), so its
    // triggered abilities are not asked.
    collect_for_objects(
        state,
        lookup,
        &state.battlefield_view(),
        events,
        true,
        &mut triggers,
    );
    collect_emblems(state, lookup, events, &mut triggers);
    graveyard_triggers(state, lookup, events, &mut triggers);
    monarch_triggers(state, events, &mut triggers);
    speed::collect(events, &mut triggers);
    radiation::collect(state, events, &mut triggers);
    sagas::collect(state, lookup, events, &mut triggers);
    cast_this_spell_triggers(state, lookup, events, &mut triggers);
    discarded::collect(state, lookup, events, &mut triggers);
    watch_triggers(state, events, &mut triggers);
    damage_watch_triggers(state, events, &mut triggers);
    attack_watch_triggers(state, events, &mut triggers);
    bird_entry::collect(state, events, &mut triggers);
    cast_copy_triggers(state, events, &mut triggers);
    collect_departed(state, lookup, events, &mut triggers);
    for trigger in &mut triggers {
        freeze_trigger(state, trigger);
        trigger.event_mana_value = trigger.event_mana_value.or_else(|| {
            trigger.event_object.and_then(|id| {
                state
                    .ltb_mana_values
                    .iter()
                    .find(|(object, _)| *object == id)
                    .map(|(_, value)| *value)
            })
        });
    }
    // Death observers froze their words at the death, before this collection.
    crate::damage_history::collect(state, lookup, from_seq, &mut triggers);
    // APNAP, then a stable timestamp order within each player's triggers.
    let active = state.turn.active;
    triggers.sort_by_key(|t| {
        let distance = (t.controller.get() + state.players.len() as u8 - active.get())
            % state.players.len() as u8;
        (distance, t.timestamp)
    });
    triggers
}

fn collect_emblems(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    // Emblems (command zone, CR 114.2): their triggered abilities fire
    // from the command zone.
    for seat in 0..state.players.len() {
        let p = PlayerId::new(seat as u8);
        for &emblem in state.zones.list(ZoneLocation::Command(p)) {
            let Some(obj) = state
                .object(emblem)
                .filter(|o| o.kind == crate::object::ObjectKind::Emblem)
            else {
                continue;
            };
            let Some(abilities) = obj.own_abilities.as_ref() else {
                continue;
            };
            for (index, ability) in abilities.iter().enumerate() {
                let Some(firing) = triggered_parts(ability) else {
                    continue;
                };
                if firing.zone != TriggerZone::Battlefield {
                    continue;
                }
                let trigger = firing.trigger;
                let list = obj.ability_list(lookup);
                let context = ability_context(state, obj, &list, index);
                // CR 603.4's first check, as in the battlefield loop below.
                for entry in events {
                    if !condition_matches(
                        state,
                        firing.condition,
                        obj.controller,
                        context,
                        Some(entry),
                    ) {
                        continue;
                    }
                    let hit = hits_with_context(
                        trigger,
                        &entry.event,
                        events,
                        state,
                        context,
                        obj.controller,
                    );
                    if hit > 0 {
                        let times = trigger_count(state, trigger, emblem, obj.controller) * hit;
                        let event_object = event_object_for(trigger, &entry.event, emblem, state);
                        let event_damage = event_damage_of(trigger, &entry.event, events);
                        for _ in 0..times {
                            triggers.push(PendingTrigger {
                                text: TextChangeMap::IDENTITY,
                                source_version: Some(obj.version),
                                related_objects: None,
                                event_subject: None,
                                event_ability: activated_stack_ability(&entry.event),
                                event_object_identity: event_object
                                    .and_then(|id| state.event_object_identity(id, entry.seq)),
                                counter_source_version: None,
                                event_mana: produced_mana(&entry.event, events),
                                event_mana_value: None,
                                event_departure: entry
                                    .departure
                                    .as_ref()
                                    .map(|d| (d.controller, d.toughness)),
                                event_amount: event_damage,
                                event_player: event_player_of(&entry.event),
                                source: emblem,
                                ability_index: index as u32,
                                abilities: Some(obj.ability_list(lookup)),
                                controller: obj.controller,
                                timestamp: obj.timestamp,
                                event_object,
                                implicit_target: None,
                                synthetic_effects: None,
                                once_per_turn: firing.once_per_turn,
                                synthetic_target: None,
                                chosen_mode: None,
                            });
                        }
                        // Once per matching event, as below.
                    }
                }
            }
        }
    }
}

fn collect_departed(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    // CR 603.10a looks back after *every* departure, including hand, library,
    // and command zone. Collect just this event batch's sources instead of
    // repeatedly scanning every accumulated graveyard and exile card.
    let mut departed: Vec<_> = events
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                ..
            }
            | GameEvent::Cycled { object, .. } => Some(object),
            _ => None,
        })
        .filter(|id| {
            state
                .object(*id)
                .is_none_or(|object| object.zone != Zone::Battlefield)
        })
        .collect();
    departed.sort_unstable();
    departed.dedup();
    collect_for_objects(state, lookup, &departed, events, false, triggers);
}

/// Printed graveyard abilities read the card as it exists there, never its
/// old battlefield abilities. A card outside the battlefield has no
/// controller, so its owner controls this trigger (CR 113.8).
fn graveyard_triggers(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for object in graveyard_observers(state, events) {
        let source = object.id;
        let owner = object.owner;
        let current = state.object_or_departed(source).unwrap_or(object);
        let list = object.ability_list(lookup);
        for (index, ability) in list.abilities.iter().enumerate() {
            let Some(firing) = triggered_parts(ability) else {
                continue;
            };
            let context = ability_context(state, object, &list, index);
            if firing.zone != TriggerZone::Graveyard {
                continue;
            }
            for entry in events {
                let at_event = source_at_event(state, current, events, entry, false);
                if at_event.zone != Zone::Graveyard
                    || at_event.version != object.version
                    || !condition_matches(state, firing.condition, owner, context, Some(entry))
                {
                    continue;
                }
                let hit =
                    hits_with_context(firing.trigger, &entry.event, events, state, context, owner);
                // These replacements describe triggered abilities of
                // permanents (CR 109.2). A graveyard card is not one,
                // even though its last controller is still stored.
                for _ in 0..hit {
                    triggers.push(PendingTrigger {
                        source,
                        text: TextChangeMap::IDENTITY,
                        source_version: Some(object.version),
                        related_objects: None,
                        event_subject: None,
                        event_ability: activated_stack_ability(&entry.event),
                        event_object_identity: event_object_for(
                            firing.trigger,
                            &entry.event,
                            source,
                            state,
                        )
                        .and_then(|id| state.event_object_identity(id, entry.seq)),
                        counter_source_version: None,
                        ability_index: index as u32,
                        abilities: Some(list.clone()),
                        controller: owner,
                        timestamp: object.timestamp,
                        event_object: event_object_for(firing.trigger, &entry.event, source, state),
                        event_mana_value: None,
                        event_departure: entry
                            .departure
                            .as_ref()
                            .map(|d| (d.controller, d.toughness)),
                        event_mana: produced_mana(&entry.event, events),
                        event_amount: event_damage_of(firing.trigger, &entry.event, events),
                        event_player: event_player_of(&entry.event),
                        implicit_target: None,
                        synthetic_effects: None,
                        synthetic_target: None,
                        once_per_turn: firing.once_per_turn,
                        chosen_mode: None,
                    });
                }
            }
        }
    }
}

/// Include graveyard incarnations which left later in this mutation batch.
/// A card moved to the graveyard after an event must not observe that earlier event.
fn graveyard_observers<'a>(
    state: &'a GameState,
    events: &[crate::event::JournalEntry],
) -> Vec<&'a crate::object::GameObject> {
    let mut objects: Vec<_> = (0..state.players.len())
        .flat_map(|seat| {
            state
                .zones
                .list(ZoneLocation::Graveyard(PlayerId::new(seat as u8)))
                .iter()
        })
        .filter_map(|id| state.object(*id))
        .filter(|object| object.is_card())
        .collect();
    for entry in events {
        let GameEvent::ZoneChanged {
            object,
            from: Zone::Graveyard,
            ..
        } = entry.event
        else {
            continue;
        };
        let Some(current) = state.object_or_departed(object) else {
            continue;
        };
        let after = source_at_event(state, current, events, entry, false);
        if let Some(before) = state.source_object(baylee_core::ids::DamageSourceRef {
            object,
            version: after.version.wrapping_sub(1),
        }) && before.zone == Zone::Graveyard
            && before.is_card()
        {
            objects.push(before);
        }
    }
    objects.sort_unstable_by_key(|object| (object.id, object.version));
    objects.dedup_by_key(|object| (object.id, object.version));
    objects
}

/// Storm and replicate retain the announced count in one instruction.
static CAST_COPIES: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::CopyThisSpell {
    amount: baylee_cards_dsl::Amount::EventAmount,
}];

/// The state triggers that trigger now (CR 603.8): every
/// [`Trigger::State`] on the battlefield whose condition holds, asked with
/// its source as `this` and the source's controller as "you".
///
/// `in_flight` answers whether the ability of `(source, index)` is already
/// waiting to go on the stack or is on it; one that is does not trigger
/// again until it has left the stack, and then only if its source is still
/// on the battlefield and the condition still holds, which asking again is.
#[must_use]
pub fn state_triggers(
    state: &GameState,
    lookup: &impl CardLookup,
    in_flight: impl Fn(ObjectId, u32) -> bool,
) -> Vec<PendingTrigger> {
    let mut triggers = Vec::new();
    for permanent in state.battlefield_seen() {
        let Some(obj) = state.object(permanent) else {
            continue;
        };
        let list = obj.ability_list(lookup);
        for (index, ability) in list.abilities.iter().enumerate() {
            let Some(firing) = triggered_parts(ability) else {
                continue;
            };
            if firing.zone != TriggerZone::Battlefield {
                continue;
            }
            let Trigger::State(condition) = firing.trigger else {
                continue;
            };
            let context = ability_context(state, obj, &list, index);
            let index = index as u32;
            if in_flight(permanent, index)
                || !eval::condition_holds_with_context(state, obj.controller, context, **condition)
                || !condition_matches(state, firing.condition, obj.controller, context, None)
            {
                continue;
            }
            triggers.push(PendingTrigger {
                text: TextChangeMap::IDENTITY,
                source_version: Some(obj.version),
                related_objects: None,
                event_subject: None,
                event_ability: None,
                event_object_identity: None,
                counter_source_version: None,
                event_mana: None,
                event_mana_value: None,
                event_departure: None,
                event_amount: None,
                event_player: None,
                source: permanent,
                ability_index: index,
                abilities: Some(list.clone()),
                controller: obj.controller,
                timestamp: obj.timestamp,
                event_object: None,
                implicit_target: None,
                synthetic_effects: None,
                once_per_turn: firing.once_per_turn,
                synthetic_target: None,
                chosen_mode: None,
            });
        }
    }
    for trigger in &mut triggers {
        freeze_trigger(state, trigger);
    }
    triggers
}

/// Storm (CR 702.40) and replicate (702.56a) trigger from casting, never copying.
/// The engine drains each mutation batch before advancing the turn. Walking
/// its cast events backwards recovers each event's earlier-spell count in
/// linear time, without rescanning the turn's entire journal for every spell.
fn cast_copy_triggers(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    if !events
        .iter()
        .any(|entry| matches!(entry.event, GameEvent::SpellCast { .. }))
    {
        return;
    }
    let mut moves = std::collections::BTreeMap::<ObjectId, u32>::new();
    let mut before = state
        .per_turn
        .spells_cast
        .iter()
        .fold(0_u32, |n, count| n.saturating_add(*count));
    let mut collected = Vec::new();
    for entry in events.iter().rev() {
        if matches!(entry.event, GameEvent::TurnStarted { .. }) {
            break;
        }
        if let GameEvent::ZoneChanged { object, .. } = entry.event {
            *moves.entry(object).or_default() += 1;
            continue;
        }
        let GameEvent::SpellCast { object, player, .. } = entry.event else {
            continue;
        };
        before = before.saturating_sub(1);
        let Some(current) = state.object(object) else {
            continue;
        };
        let reference = baylee_core::ids::DamageSourceRef {
            object,
            version: current
                .version
                .wrapping_sub(moves.get(&object).copied().unwrap_or(0)),
        };
        let spell = state.source_object(reference).unwrap_or(current);
        let storm = spell
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::STORM);
        for copies in [
            storm.then_some(before),
            (spell.replicated > 0).then_some(u32::from(spell.replicated)),
        ]
        .into_iter()
        .flatten()
        {
            collected.push(PendingTrigger {
                text: TextChangeMap::IDENTITY,
                source_version: Some(spell.version),
                related_objects: None,
                event_subject: None,
                event_ability: None,
                event_object_identity: None,
                counter_source_version: None,
                event_amount: Some(copies),
                event_player: None,
                event_mana: None,
                event_mana_value: None,
                event_departure: None,
                source: object,
                ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                abilities: None,
                controller: player,
                timestamp: spell.timestamp,
                event_object: Some(object),
                implicit_target: Some(object),
                synthetic_effects: Some(CAST_COPIES),
                once_per_turn: false,
                synthetic_target: None,
                chosen_mode: None,
            });
        }
    }
    triggers.extend(collected.into_iter().rev());
}

/// "At the beginning of the monarch's end step, that player draws a card."
static MONARCH_DRAW: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::draw(1)];

/// "Whenever a creature deals combat damage to the monarch, its controller
/// becomes the monarch." The creature is the ability's event object, which
/// the synthetic path also puts first among its objects, and that is what
/// `ControllerOfTarget` reads.
static MONARCH_TAKEOVER: &[baylee_cards_dsl::Effect] = &[baylee_cards_dsl::Effect::BecomeMonarch(
    PlayerRel::ControllerOfTarget,
)];

/// The monarch's two inherent triggered abilities (CR 724.2).
///
/// No permanent has them, so no walk over permanents finds them: they are
/// read off the events directly. They have no source ([`ObjectId::NO_SOURCE`])
/// and are controlled by whoever was the monarch when they triggered, which
/// for the takeover is the player who is about to lose the title.
///
/// Delayed triggered abilities that watch an object (CR 603.7): earthbend's
/// "when that land dies or is put into exile, return it to the battlefield
/// tapped under your control" (CR 701.66a).
///
/// Each watch triggers on the first time its object leaves the battlefield
/// after the watch was created (CR 603.7a, 603.7b), and only if that was to
/// a graveyard or into exile: a land bounced to its owner's hand has left
/// and is a new object, and the watch is spent without triggering. The
/// scan that follows removes every watch whose object is gone
/// (`Engine::queue_new_triggers`), so none is read twice.
///
/// The event object is the card, which is what "return it" reads
/// (`TargetSpec::EventObject`); the source and controller are the ones the
/// watch was created with (CR 603.7d, 603.7e).
#[allow(clippy::too_many_lines)] // one flat read of each watch kind
fn watch_triggers(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for watch in &state.delayed {
        // "When that land dies or is put into exile" watches two
        // destinations; "when this Aura leaves the battlefield" every one.
        let (card, after, to_graveyard_only, any_zone) = match watch.when {
            crate::state::DelayedWhen::DiesOrIsExiled { card, after, .. } => {
                (card, after, false, false)
            }
            crate::state::DelayedWhen::DiesThisTurn { card, after, .. } => {
                (card, after, true, false)
            }
            crate::state::DelayedWhen::LeavesBattlefield { card, after, .. } => {
                (card, after, false, true)
            }
            _ => continue,
        };
        // A watch about another object ("that creature's controller
        // sacrifices it") carries that object as its event object, as the
        // incarnation it was (CR 603.7c, 400.7).
        let (source, source_version, effects, text, about) = match watch.action {
            crate::state::DelayedAction::Trigger {
                source,
                source_version,
                effects,
                text,
            } => (source, source_version, effects, text, None),
            crate::state::DelayedAction::TriggerAbout {
                source,
                source_version,
                effects,
                text,
                object,
                version,
                ..
            } => (
                source,
                source_version,
                effects,
                text,
                Some((object, version)),
            ),
            _ => continue,
        };
        let left = events
            .iter()
            .filter(|entry| entry.seq > after)
            .find_map(|entry| match entry.event {
                GameEvent::ZoneChanged {
                    object,
                    from: Zone::Battlefield,
                    to,
                    ..
                } if object == card => Some(to),
                _ => None,
            });
        let fires = if any_zone {
            left.is_some()
        } else if to_graveyard_only {
            matches!(left, Some(Zone::Graveyard))
        } else {
            matches!(left, Some(Zone::Graveyard | Zone::Exile))
        };
        if !fires {
            continue;
        }
        let (event_object, event_object_identity) = match about {
            Some((object, version)) => (
                Some(object),
                Some((
                    version,
                    state
                        .object(object)
                        .and_then(|o| o.characteristics().power)
                        .unwrap_or(0),
                )),
            ),
            // The card as it is right after it left: the incarnation the
            // trigger is about (CR 400.7), so a return reads that one and
            // not a later one.
            None => (
                Some(card),
                state
                    .object(card)
                    .map(|o| (o.version, o.characteristics().power.unwrap_or(0))),
            ),
        };
        triggers.push(PendingTrigger {
            text,
            source_version: Some(source_version),
            event_ability: None,
            event_object_identity,
            related_objects: None,
            event_subject: None,
            counter_source_version: None,
            event_amount: None,
            event_player: None,
            event_mana: None,
            event_mana_value: None,
            event_departure: None,
            source,
            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
            abilities: None,
            controller: watch.controller,
            timestamp: state.object(source).map_or(0, |o| o.timestamp),
            event_object,
            implicit_target: None,
            synthetic_effects: Some(effects),
            once_per_turn: false,
            synthetic_target: None,
            chosen_mode: None,
        });
    }
}

/// "Whenever that creature is dealt damage by [a source the filter matches]
/// this turn" (Glyph of Life, `DelayedWhen::DealtDamageThisTurn`): once for
/// each such damage event after the watch began, while the watched object is
/// still the one it was. The source is asked as it deals the damage — the
/// scan follows the event before anything else happens — with the watch's
/// controller as "you". "That much" is the event's amount.
fn damage_watch_triggers(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for watch in &state.delayed {
        let crate::state::DelayedWhen::DealtDamageThisTurn {
            card,
            version,
            by,
            after,
        } = watch.when
        else {
            continue;
        };
        let crate::state::DelayedAction::Trigger {
            source,
            source_version,
            effects,
            text,
        } = watch.action
        else {
            continue;
        };
        if state.object(card).is_none_or(|o| o.version != version) {
            continue;
        }
        for entry in events.iter().filter(|entry| entry.seq > after) {
            let GameEvent::DamageDealt {
                source: Some(damager),
                target: crate::event::DamageTarget::Object(recipient),
                amount,
                ..
            } = entry.event
            else {
                continue;
            };
            if recipient != card || amount == 0 {
                continue;
            }
            let Some(dealer) = state.object_or_departed(damager) else {
                continue;
            };
            if !eval::matches(by, state, dealer, watch.controller, source) {
                continue;
            }
            triggers.push(PendingTrigger {
                text,
                source_version: Some(source_version),
                event_ability: None,
                event_object_identity: None,
                related_objects: None,
                event_subject: None,
                counter_source_version: None,
                event_amount: Some(amount),
                event_player: None,
                event_mana: None,
                event_mana_value: None,
                event_departure: None,
                source,
                ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                abilities: None,
                controller: watch.controller,
                timestamp: state.object(source).map_or(0, |o| o.timestamp),
                event_object: Some(damager),
                implicit_target: None,
                synthetic_effects: Some(effects),
                once_per_turn: false,
                synthetic_target: None,
                chosen_mode: None,
            });
        }
    }
}

/// "Whenever you attack this turn" (Dalkovan Encampment,
/// `DelayedWhen::YouAttackThisTurn`): once for each declaration of attackers
/// after the watch began that holds at least one creature of the watch's
/// controller. One declaration is one action, and the scan reads each
/// action's events once, so a scan triggers each watch at most once; a
/// creature put onto the battlefield attacking journals no `BecameAttacker`
/// (CR 508.3a).
fn attack_watch_triggers(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for watch in &state.delayed {
        let crate::state::DelayedWhen::YouAttackThisTurn { after } = watch.when else {
            continue;
        };
        let crate::state::DelayedAction::Trigger {
            source,
            source_version,
            effects,
            text,
        } = watch.action
        else {
            continue;
        };
        let attacked = events
            .iter()
            .filter(|entry| entry.seq > after)
            .any(|entry| {
                matches!(entry.event, GameEvent::BecameAttacker { object, .. }
                if state.object(object).is_some_and(|o| o.controller == watch.controller))
            });
        if attacked {
            triggers.push(PendingTrigger {
                text,
                source_version: Some(source_version),
                event_ability: None,
                event_object_identity: None,
                related_objects: None,
                event_subject: None,
                counter_source_version: None,
                event_amount: None,
                event_player: Some(watch.controller),
                event_mana: None,
                event_mana_value: None,
                event_departure: None,
                source,
                ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                abilities: None,
                controller: watch.controller,
                timestamp: state.object(source).map_or(0, |o| o.timestamp),
                event_object: None,
                implicit_target: None,
                synthetic_effects: Some(effects),
                once_per_turn: false,
                synthetic_target: None,
                chosen_mode: None,
            });
        }
    }
}

/// Whether a `ManaProduced` event is the first mana of an activation that
/// tapped `object` as its cost: "tap [a permanent] for mana" is activating a
/// mana ability of it with {T} in the cost (CR 106.12), and the trigger
/// fires as that ability resolves and produces mana (CR 106.12a).
///
/// Read off the journal, where every mana ability with {T} writes the same
/// pair — the tap under `Cause::Cost`, then the mana, the intrinsic door
/// (`casting::add_intrinsic_mana`) and the resolved ability (`resolve::mana`)
/// alike. The nearest earlier entry about `object` decides: its own tap
/// means this is the tap's first mana, and its own earlier mana means this
/// is the second colour of one activation ("Add {R}{G}"), which is still
/// one tap. A tap to attack is `Cause::TurnBased` and a creature tapped for
/// convoke or crew makes no mana, so neither reaches here.
fn first_mana_of_a_tap(
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
    object: ObjectId,
) -> bool {
    let Some(at) = event_groups::index_of_event(event, batch) else {
        return false;
    };
    batch[..at]
        .iter()
        .rev()
        .find_map(|entry| match entry.event {
            GameEvent::AbilityActivated {
                source,
                tap_cost,
                mana_ability,
                ..
            } if source.object == object => Some(tap_cost && mana_ability),
            GameEvent::ObjectTapped {
                object: tapped,
                cause: crate::event::Cause::Cost,
            } if tapped == object => Some(true),
            GameEvent::ManaProduced {
                source: Some(made), ..
            } if made == object => Some(false),
            _ => None,
        })
        .unwrap_or(false)
}

/// Capture all types made by this activation, stopping at the next tap of
/// the same source. Bonus mana from another permanent is not its production.
fn produced_mana(event: &GameEvent, batch: &[crate::event::JournalEntry]) -> Option<EventMana> {
    let GameEvent::ManaProduced {
        player,
        source: Some(source),
        ..
    } = event
    else {
        return None;
    };
    let at = event_groups::index_of_event(event, batch)?;
    let mut types = 0;
    for entry in &batch[at..] {
        match entry.event {
            GameEvent::ObjectTapped { object, .. } if object == *source => break,
            GameEvent::ManaProduced {
                source: Some(object),
                player: who,
                color,
                amount,
            } if object == *source && who == *player && amount > 0 => {
                types |= 1 << (color as u8);
            }
            _ => {}
        }
    }
    Some(EventMana {
        player: *player,
        types,
    })
}

/// Whether a combat `DamageDealt` is the first of its combat damage step to
/// reach `dealt` — a permanent or a player. All combat damage in a step is
/// dealt at once (CR 510.2), so however many creatures dealt it, `dealt`
/// was dealt damage in one event, and "whenever this creature is dealt
/// damage" or "whenever you're dealt damage" triggers once (CR 603.2c).
/// One step's damage is one batch: nothing is scanned between its events,
/// and first strike's damage is another step.
fn first_combat_damage_to(
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
    dealt: crate::event::DamageTarget,
) -> bool {
    let Some(at) = event_groups::index_of_event(event, batch) else {
        return false;
    };
    !batch[..at].iter().any(|entry| {
        matches!(
            entry.event,
            GameEvent::DamageDealt {
                target,
                amount,
                is_combat: true,
                ..
            } if target == dealt && amount > 0
        )
    })
}

/// All the combat damage a batch dealt to `player`: the one event
/// [`first_combat_damage_to`] fires on, as an amount (CR 510.2).
fn combat_damage_in_batch(batch: &[crate::event::JournalEntry], player: PlayerId) -> u32 {
    batch
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::DamageDealt {
                target: crate::event::DamageTarget::Player(to),
                amount,
                is_combat: true,
                ..
            } if to == player => Some(amount),
            _ => None,
        })
        .fold(0u32, u32::saturating_add)
}

/// All the combat damage `source` dealt in a batch, to every recipient
/// (CR 510.2): the one event [`Trigger::DealsDamage`] fires on.
fn combat_damage_by_in_batch(batch: &[crate::event::JournalEntry], source: ObjectId) -> u32 {
    batch
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::DamageDealt {
                source: Some(from),
                amount,
                is_combat: true,
                ..
            } if from == source => Some(amount),
            _ => None,
        })
        .fold(0u32, u32::saturating_add)
}

/// Whether `event` is the first combat damage `source` dealt in `batch`.
fn first_combat_damage_by(
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
    source: ObjectId,
) -> bool {
    let Some(at) = event_groups::index_of_event(event, batch) else {
        return false;
    };
    !batch[..at].iter().any(|entry| {
        matches!(
            entry.event,
            GameEvent::DamageDealt {
                source: Some(from),
                amount,
                is_combat: true,
                ..
            } if from == source && amount > 0
        )
    })
}

/// The monarch is read once for the whole batch. A batch is what happened
/// between two scans, and nothing that makes a player the monarch shares
/// one with a step beginning or with combat damage: a resolution is scanned
/// before the next step begins, and damage is scanned before anything
/// resolves.
///
/// Timestamp `0` puts them ahead of the monarch's other triggers from the
/// same batch in the queue, so they go on the stack first and resolve last.
/// The rules let the controller choose that order (CR 603.3b), which this
/// engine does not ask yet for any trigger.
fn monarch_triggers(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    let Some(monarch) = state.monarch else {
        return;
    };
    let inherent = |effects, event_object| PendingTrigger {
        text: TextChangeMap::IDENTITY,
        source_version: None,
        related_objects: None,
        event_subject: None,
        event_ability: None,
        event_object_identity: None,
        counter_source_version: None,
        event_mana: None,
        event_mana_value: None,
        event_departure: None,
        event_amount: None,
        event_player: None,
        source: ObjectId::NO_SOURCE,
        ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
        abilities: None,
        controller: monarch,
        timestamp: 0,
        event_object,
        implicit_target: event_object,
        synthetic_effects: Some(effects),
        once_per_turn: false,
        synthetic_target: None,
        chosen_mode: None,
    };
    for entry in events {
        match &entry.event {
            GameEvent::StepChanged {
                step: crate::turn::Step::End,
                ..
            } if state.turn.active == monarch => {
                triggers.push(inherent(MONARCH_DRAW, None));
            }
            GameEvent::DamageDealt {
                source: Some(creature),
                target: crate::event::DamageTarget::Player(player),
                is_combat: true,
                ..
            } if *player == monarch
                && state.object_or_departed(*creature).is_some_and(|o| {
                    o.characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
                }) =>
            {
                triggers.push(inherent(MONARCH_TAKEOVER, Some(*creature)));
            }
            _ => {}
        }
    }
}

static PROWESS_SELF: baylee_cards_dsl::Filter = baylee_cards_dsl::Filter::This;

static PROWESS_PUMP: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::CreateContinuousEffect {
        layer: baylee_cards_dsl::Layer::PtModify,
        filter: &PROWESS_SELF,
        modifier: baylee_cards_dsl::Modifier::ModifyPT(1, 1),
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
    }];

/// Ward {2} fallback: counter the targeting spell/ability (the implicit
/// first target).
static WARD_COUNTER: baylee_cards_dsl::Effect =
    baylee_cards_dsl::Effect::CounterTargetSpellOrAbility;
const fn ward_pay_or_counter(n: u32) -> [baylee_cards_dsl::Effect; 1] {
    [baylee_cards_dsl::Effect::PlayerMayPayOr {
        player: baylee_cards_dsl::PlayerRel::ControllerOfTarget,
        mana: baylee_cards_dsl::Amount::Fixed(n),
        effect: &WARD_COUNTER,
    }]
}

/// One synthetic effect list per generic ward cost, **indexed by the cost**.
///
/// It was three hand-written statics for ward {1}, {2} and {3}, with anything
/// else falling through a `continue` — and Tyrranax Rex prints ward {4}, so
/// the card sat there `Coverage::Partial` about its toxic and silently
/// carrying no ward at all. A table indexed by the number cannot go one
/// short the next time a set prints a bigger one, and the ceiling is still a
/// refusal rather than a guess: a cost above this is skipped, exactly as a
/// *coloured* ward is, because `Amount::Fixed` says generic mana and nothing
/// else.
static WARD_PAY_OR_COUNTER: [[baylee_cards_dsl::Effect; 1]; 11] = [
    ward_pay_or_counter(0),
    ward_pay_or_counter(1),
    ward_pay_or_counter(2),
    ward_pay_or_counter(3),
    ward_pay_or_counter(4),
    ward_pay_or_counter(5),
    ward_pay_or_counter(6),
    ward_pay_or_counter(7),
    ward_pay_or_counter(8),
    ward_pay_or_counter(9),
    ward_pay_or_counter(10),
];

fn begins_step(
    state: &GameState,
    began: crate::turn::Step,
    expected: baylee_cards_dsl::StepKind,
) -> bool {
    if expected == baylee_cards_dsl::StepKind::FirstMain {
        began == crate::turn::Step::Main && state.per_turn.main_phases_begun == 1
    } else {
        began.kind() == Some(expected)
    }
}

fn cast_mana_value(state: &GameState, event: &GameEvent) -> Option<u32> {
    let GameEvent::SpellCast { object, .. } = event else {
        return None;
    };
    let spell = state.object(*object)?;
    let chars = spell.characteristics();
    Some(
        chars
            .front_mana_value
            .map_or_else(|| chars.mana_cost.with_x(spell.x_value).cmc(), u32::from),
    )
}

/// The largest generic ward cost [`WARD_PAY_OR_COUNTER`] can charge.
///
/// Read by `keyword_tests::every_ward_in_the_pool_is_one_the_engine_charges`,
/// which is what turns "the table is long enough" from a thing somebody
/// remembers into a build failure the day a set prints a bigger one.
#[cfg(test)]
pub(crate) const WARD_CEILING: usize = WARD_PAY_OR_COUNTER.len() - 1;

/// How much damage a damage event dealt, as `trigger` reads it: to a
/// player, or for [`Trigger::DealsDamage`] to anything.
///
/// Per entry, except where the trigger is about the player rather than the
/// source: [`Trigger::PlayerDealtDamage`] fires once for a step's combat
/// damage (CR 510.2, 603.2c), and "that many" is all of it, so its amount
/// is the batch's total to that player. A source's trigger
/// (`DealsCombatDamageToOpponent`) keeps its own share: two creatures are
/// two sources, each dealing its own damage. [`Trigger::DealsDamage`] is
/// the source's side of that: once for a step's combat damage, so "that
/// much" is all the combat damage the source dealt in the step, to
/// whatever it was dealt.
fn event_damage_of(
    trigger: &Trigger,
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
) -> Option<u32> {
    match event {
        GameEvent::DamageDealt {
            source: Some(source),
            is_combat: true,
            ..
        } if matches!(trigger, Trigger::DealsDamage(_)) => {
            Some(combat_damage_by_in_batch(batch, *source))
        }
        GameEvent::DamageDealt { amount, .. } if matches!(trigger, Trigger::DealsDamage(_)) => {
            Some(*amount)
        }
        GameEvent::DamageDealt {
            target: crate::event::DamageTarget::Player(player),
            is_combat: true,
            ..
        } if matches!(trigger, Trigger::PlayerDealtDamage(_)) => {
            Some(combat_damage_in_batch(batch, *player))
        }
        GameEvent::DamageDealt {
            target: crate::event::DamageTarget::Player(_),
            amount,
            ..
        } => Some(*amount),
        _ => None,
    }
}

/// The player a triggering event is about: "that player" or "they" of the
/// ability it triggers (`PlayerRel::EventPlayer`, CR 603.2).
///
/// The one who drew, cast the spell (CR 112.2: the player who put it on the
/// stack), played the land, cycled or discarded the card, whose life
/// changed, whose pool the mana went to, or who was dealt the damage. An
/// event that is no player's, or one about an object (whose controller is
/// `PlayerRel::ControllerOfEvent`), has none. Read here, as the trigger is
/// collected, because by the time the ability resolves the event is gone:
/// a draw leaves no object behind to ask.
fn event_player_of(event: &GameEvent) -> Option<PlayerId> {
    match event {
        GameEvent::AbilityActivated { player, .. }
        | GameEvent::CardsDrawn { player, .. }
        | GameEvent::SpellCast { player, .. }
        | GameEvent::SpellCopied { player, .. }
        | GameEvent::LandPlayed { player, .. }
        | GameEvent::Cycled { player, .. }
        | GameEvent::Discarded { player, .. }
        | GameEvent::LifeChanged { player, .. }
        | GameEvent::ManaProduced { player, .. }
        | GameEvent::DamageDealt {
            target: crate::event::DamageTarget::Player(player),
            ..
        } => Some(*player),
        _ => None,
    }
}

/// The object a trigger's event is about, as the triggered ability reads it
/// ([`TargetSpec::EventObject`](baylee_cards_dsl::TargetSpec)).
///
/// The event decides, except where one event names two objects and the
/// trigger says which it means. A blocker's declaration names the blocker
/// and the creature it blocks; "whenever this creature blocks or becomes
/// blocked by a non-Wall creature, destroy that creature" is about the one
/// that is not the source, whichever side of the block that is.
fn event_object_for(
    trigger: &Trigger,
    event: &GameEvent,
    source: ObjectId,
    state: &GameState,
) -> Option<ObjectId> {
    match (trigger, event) {
        // The other creature of the pair: the one that is not the creature
        // this source is attached to.
        (
            Trigger::AttachedBlocksOrBecomesBlockedBy(_),
            GameEvent::BecameBlocker {
                object: blocker,
                attacker,
            },
        ) => {
            let host = state.object(source)?.attached_to?;
            Some(if *blocker == host {
                *attacker
            } else {
                *blocker
            })
        }
        (
            Trigger::BecomesBlockedBy(_) | Trigger::BecomesBlocked,
            GameEvent::BecameBlocker {
                object: blocker, ..
            },
        ) => Some(*blocker),
        (
            Trigger::DealsCombatDamageToCreature(_),
            GameEvent::DamageDealt {
                target: crate::event::DamageTarget::Object(recipient),
                ..
            },
        ) => Some(*recipient),
        (
            Trigger::DealsDamageToPlayer { .. }
            | Trigger::OneOrMoreDealCombatDamageToPlayer(_)
            | Trigger::DealsCombatDamageToYou(_),
            GameEvent::DamageDealt {
                source: damage_source,
                ..
            },
        ) => *damage_source,
        (Trigger::BlocksCreature(_), GameEvent::BecameBlocker { attacker, .. }) => Some(*attacker),
        (
            Trigger::BlocksOrBecomesBlockedBy(_),
            GameEvent::BecameBlocker {
                object: blocker,
                attacker,
            },
        ) => Some(if *blocker == source {
            *attacker
        } else {
            *blocker
        }),
        _ => event_object_of(event),
    }
}

/// The creature a pair trigger about an attached creature is about
/// (`Trigger::AttachedBlocksOrBecomesBlockedBy`), as the object it is now:
/// the one of the pair this source is attached to.
fn event_subject_for(
    trigger: &Trigger,
    event: &GameEvent,
    source: ObjectId,
    state: &GameState,
) -> Option<baylee_core::ids::DamageSourceRef> {
    let (
        Trigger::AttachedBlocksOrBecomesBlockedBy(_),
        GameEvent::BecameBlocker {
            object: blocker,
            attacker,
        },
    ) = (trigger, event)
    else {
        return None;
    };
    let host = state.object(source)?.attached_to?;
    if host != *blocker && host != *attacker {
        return None;
    }
    let version = state.object(host)?.version;
    Some(baylee_core::ids::DamageSourceRef {
        object: host,
        version,
    })
}

/// The object an event is about, if any.
fn event_object_of(event: &GameEvent) -> Option<ObjectId> {
    match event {
        GameEvent::AbilityActivated { source, .. } => Some(source.object),
        GameEvent::ZoneChanged { object, .. }
        | GameEvent::SpellCast { object, .. }
        | GameEvent::SpellCopied { object, .. }
        | GameEvent::Discarded { object, .. }
        | GameEvent::CounterChanged { object, .. }
        | GameEvent::AbilityTriggered { object, .. }
        | GameEvent::BecameTarget { object, .. }
        | GameEvent::PlayerBecameTarget { object, .. }
        | GameEvent::BecameAttacker { object, .. }
        | GameEvent::BecameBlocker { object, .. }
        | GameEvent::AttackerBlocked { object }
        | GameEvent::AttackerUnblocked { object }
        // The permanent that became tapped: "that land's controller" of
        // Psychic Venom.
        | GameEvent::ObjectTapped { object, .. }
        // The permanent tapped for mana (CR 106.12a): "its controller" and
        // "that player" of Gauntlet of Might and Manabarbs.
        | GameEvent::ManaProduced {
            source: Some(object),
            ..
        } => Some(*object),
        _ => None,
    }
}

/// The stack object that has just acquired this target, and its controller.
/// Retargeting names only newly acquired targets; retaining one never retriggers ward.
fn targeting(
    event: &GameEvent,
    state: &GameState,
    target: ObjectId,
) -> Option<(ObjectId, PlayerId)> {
    match *event {
        GameEvent::BecameTarget {
            object,
            target: acquired,
            controller,
        } => (acquired == target).then_some((object, controller)),
        GameEvent::SpellCast {
            object,
            player: controller,
            ..
        }
        | GameEvent::AbilityTriggered {
            object, controller, ..
        } => state
            .object(object)
            .filter(|_| state.targets_current_object(object, target))
            .map(|_| (object, controller)),
        _ => None,
    }
}

/// "When you cast this spell" (cascade, CR 702.85a): a trigger condition
/// that cannot trigger from the battlefield functions where it can, which is
/// the stack (CR 113.6k). Each spell cast in this batch is asked for its own
/// `Trigger::SpellCast(&Filter::This)` and mana-qualified self-cast abilities — its
/// other abilities do not function there.
fn cast_this_spell_triggers(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for entry in events {
        let GameEvent::SpellCast { object, player, .. } = entry.event else {
            continue;
        };
        let Some(spell) = state.object(object).filter(|o| o.zone == Zone::Stack) else {
            continue;
        };
        let list = spell.ability_list(lookup);
        for (index, ability) in list.abilities.iter().enumerate() {
            let Some(firing) = triggered_parts(ability) else {
                continue;
            };
            let cast_this = match firing.trigger {
                Trigger::SpellCast(baylee_cards_dsl::Filter::This) => true,
                Trigger::CastThisWithMana(color) => spell
                    .paid
                    .as_ref()
                    .is_some_and(|paid| paid.mana_types_spent[color.index()] > 0),
                _ => false,
            };
            if firing.zone != TriggerZone::Battlefield || !cast_this {
                continue;
            }
            let context = ability_context(state, spell, &list, index);
            if !condition_matches(state, firing.condition, player, context, None) {
                continue;
            }
            triggers.push(PendingTrigger {
                text: TextChangeMap::IDENTITY,
                source_version: Some(spell.version),
                related_objects: None,
                event_subject: None,
                event_ability: None,
                event_object_identity: None,
                counter_source_version: None,
                event_mana: None,
                event_mana_value: None,
                event_departure: None,
                source: object,
                ability_index: index as u32,
                abilities: Some(list.clone()),
                controller: player,
                timestamp: spell.timestamp,
                event_object: Some(object),
                implicit_target: None,
                synthetic_effects: None,
                once_per_turn: firing.once_per_turn,
                synthetic_target: None,
                chosen_mode: None,
                event_amount: None,
                event_player: None,
            });
        }
    }
}

/// How many times one journal entry fires `trigger` for this source.
///
/// Once for each happening the entry records ([`repeats`]) when [`matches`]
/// says so, for every trigger but two. The one that counts what an event
/// targeted: "whenever you or a permanent you control becomes the target of a
/// spell or ability an opponent controls" fires once for each of them, so one
/// spell aimed at you and at a creature of yours fires it twice. And the one
/// that watches all of an entry's draws but one: "whenever an opponent draws
/// a card except the first one they draw in each of their draw steps" skips
/// the step's first card and no other, so "draw three" that opens the step
/// fires it twice. That is a subtraction, which is why the count of
/// happenings is taken here rather than multiplied in by the callers.
#[cfg(test)]
fn hits(
    trigger: &Trigger,
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
    state: &GameState,
    source: ObjectId,
    you: PlayerId,
) -> u32 {
    hits_with_context(
        trigger,
        event,
        batch,
        state,
        eval::live_context(state, source),
        you,
    )
}

fn hits_with_context(
    trigger: &Trigger,
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
    state: &GameState,
    context: RuleContext,
    you: PlayerId,
) -> u32 {
    match trigger {
        Trigger::TargetedByOpponent {
            filter,
            you: counts_you,
        } => targeted_by_opponent(event, state, filter, *counts_you, context, you) * repeats(event),
        Trigger::DrawsExceptFirst(rel) => match event {
            GameEvent::CardsDrawn {
                player,
                first_in_draw_step,
                ..
            } if match rel {
                PlayerRel::You => *player == you,
                PlayerRel::Opponent | PlayerRel::EachOpponent => state.is_opponent(*player, you),
                _ => true,
            } =>
            {
                // A card drawn on somebody else's turn, or in their own
                // upkeep, is in none of their draw steps and always fires:
                // that is what the card is played for (Brainstorm, Rhystic
                // Study, a Howling Mine on my turn). Which entry holds the
                // step's one excepted card is written at the draw, because
                // a count of the turn's cards read here, at collection,
                // could not tell it apart.
                repeats(event).saturating_sub(u32::from(*first_in_draw_step))
            }
            _ => 0,
        },
        _ => u32::from(matches(trigger, event, batch, state, context, you)) * repeats(event),
    }
}

fn departure_hit(
    trigger: &Trigger,
    entry: &crate::event::JournalEntry,
    source: ObjectId,
) -> Option<u32> {
    if matches!(
        *trigger,
        Trigger::Dies(&baylee_cards_dsl::Filter::AttachedToBySource)
    ) && matches!(
        entry.event,
        GameEvent::ZoneChanged {
            from: Zone::Battlefield,
            to: Zone::Graveyard,
            ..
        }
    ) {
        return entry
            .departure
            .as_ref()
            .map(|d| u32::from(d.attachments.contains(&source)));
    }
    None
}

/// A leaves-the-battlefield trigger's filter, asked of the object as it
/// last existed on the battlefield (CR 603.10a): its projected
/// characteristics and its counters as it left, where the state still has
/// them, and the object as it is otherwise.
///
/// The counters are written onto a copy because `move_object` empties them
/// on the way out, so "with a -1/-1 counter on it" (The Reaper, King No
/// More) was never true of the card in the graveyard. A copy per departed
/// object and trigger source, which only a departure from the battlefield
/// pays for. The controller needs no such help here: the field still holds
/// the controller the permanent left with while triggers are collected, and
/// only a later refresh settles it back to the owner, which is why
/// `ltb_controllers` exists for what reads it at resolution.
fn departed_matches(
    filter: &baylee_cards_dsl::Filter,
    state: &GameState,
    object: ObjectId,
    you: PlayerId,
    context: RuleContext,
) -> bool {
    let Some(o) = state.object_or_departed(object) else {
        return false;
    };
    let Some(was) = state.last_known_characteristics(object) else {
        return eval::matches_with_context(filter, state, o, you, context);
    };
    let mut as_it_was = o.clone();
    as_it_was.counters = state
        .ltb_counters
        .iter()
        .find(|(id, _)| *id == object)
        .map(|(_, counters)| counters.clone())
        .unwrap_or_default();
    eval::matches_projected_with_context(filter, state, &as_it_was, was, you, context)
}

/// [`Trigger::TargetedByOpponent`]'s count: the fitting targets an
/// opponent's spell or ability acquired in this event.
///
/// A cast or an activation announces its targets in its own event, and a
/// copy or a retargeting effect journals each target it newly acquired
/// ([`GameEvent::BecameTarget`], [`GameEvent::PlayerBecameTarget`]). An
/// object counts only while it is a permanent: a card in a graveyard a spell
/// targets is not "a permanent you control".
fn targeted_by_opponent(
    event: &GameEvent,
    state: &GameState,
    filter: &baylee_cards_dsl::Filter,
    counts_you: bool,
    context: RuleContext,
    you: PlayerId,
) -> u32 {
    let fits = |target: ObjectId| {
        state.object(target).is_some_and(|o| {
            o.zone == Zone::Battlefield
                && eval::matches_with_context(filter, state, o, you, context)
        })
    };
    match *event {
        GameEvent::BecameTarget {
            target, controller, ..
        } => u32::from(state.is_opponent(controller, you) && fits(target)),
        GameEvent::PlayerBecameTarget {
            player, controller, ..
        } => u32::from(counts_you && player == you && state.is_opponent(controller, you)),
        GameEvent::SpellCast {
            object,
            player: controller,
            ..
        }
        | GameEvent::AbilityTriggered {
            object, controller, ..
        } => {
            if !state.is_opponent(controller, you) {
                return 0;
            }
            let Some(obj) = state.object(object) else {
                return 0;
            };
            let mut targets: Vec<ObjectId> = obj
                .targets
                .iter()
                .chain(obj.second_targets())
                .copied()
                .collect();
            targets.sort_unstable();
            targets.dedup();
            let objects = targets
                .into_iter()
                .filter(|t| state.targets_current_object(object, *t) && fits(*t))
                .count();
            let player =
                counts_you && (obj.target_players.contains(you) || obj.chosen_player == Some(you));
            u32::try_from(objects).unwrap_or(u32::MAX) + u32::from(player)
        }
        _ => 0,
    }
}

/// The source and the event object are different identities. A death's event
/// object is the card in its destination (CR 400.7e); the ability's source is
/// the permanent immediately before departure (CR 113.7a, 603.10a).
fn source_at_event<'a>(
    state: &'a GameState,
    current: &'a crate::object::GameObject,
    events: &[crate::event::JournalEntry],
    event: &crate::event::JournalEntry,
    look_back: bool,
) -> &'a crate::object::GameObject {
    let subsequent_moves = events.iter().filter(|entry| {
        entry.seq > event.seq
            && matches!(entry.event, GameEvent::ZoneChanged { object, .. } if object == current.id)
    }).count();
    let version = current
        .version
        .wrapping_sub(u32::try_from(subsequent_moves).expect("zone changes fit version"));
    let at_event = state
        .source_object(baylee_core::ids::DamageSourceRef {
            object: current.id,
            version,
        })
        .unwrap_or(current);
    if !look_back || at_event.zone == crate::zone::Zone::Battlefield {
        return at_event;
    }
    // Simultaneous departures have separate journal rows. The source may
    // precede another creature whose death it observes; use its own departure,
    // never the victim's version or an unrelated later incarnation.
    events.iter().rev().filter(|entry| entry.seq <= event.seq).find_map(|entry| {
        if matches!(entry.event, GameEvent::ZoneChanged { object, from: crate::zone::Zone::Battlefield, .. } if object == current.id) {
            entry.departure.as_ref().and_then(|departure| state.source_object(baylee_core::ids::DamageSourceRef { object: current.id, version: departure.version }))
        } else {
            None
        }
    }).unwrap_or(at_event)
}

/// Scans a set of objects for triggered abilities matching the events.
/// `all_kinds` = every trigger kind (battlefield scan); `false` = only
/// LTB/Dies (off-battlefield scan, CR 603.10).
#[allow(clippy::too_many_lines)]
fn collect_for_objects(
    state: &GameState,
    lookup: &impl CardLookup,
    objects: &[ObjectId],
    events: &[crate::event::JournalEntry],
    all_kinds: bool,
    triggers: &mut Vec<PendingTrigger>,
) {
    // Most temporary effects only change stats or keywords. Select grants
    // once per scan, not once for every permanent in a token army.
    let granted: smallvec::SmallVec<[&crate::effects::ContinuousEffect; 4]> = state
        .effects
        .iter()
        .filter(|fx| {
            matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::GrantTriggered { .. }
            )
        })
        .collect();
    for &permanent in objects {
        // The look-back applies only to sources that left during this
        // event batch. A card already in a graveyard has no battlefield
        // ability to observe a later death. Keep the whole batch so a
        // dying source still observes creatures dying alongside it. The one
        // other card that triggers from here is one cycled in this batch:
        // "when you cycle this card" triggers from wherever the card winds
        // up (CR 702.29c), and it left a hand, not the battlefield.
        if !all_kinds
            && !events.iter().any(|entry| match entry.event {
                GameEvent::ZoneChanged {
                    object,
                    from: Zone::Battlefield,
                    ..
                }
                | GameEvent::Cycled { object, .. } => object == permanent,
                _ => false,
            })
        {
            continue;
        }
        // `object_or_departed` and not `object`: with `all_kinds` false this
        // is the look-back scan, and one of the lists it walks is the objects
        // that have ceased to exist (CR 111.7). On the battlefield pass the
        // two are the same function — nothing on the battlefield has ceased.
        let Some(obj) = state.object_or_departed(permanent) else {
            continue;
        };
        // CR 603.10a: the look-back scan asks an object that has already
        // arrived somewhere else, and what it must see is the abilities that
        // existed "immediately prior to the event". A copy gives its rules
        // text back on the way out (CR 400.7), so `abilities` would answer
        // with the printed card — which is how a Phyrexian Metamorph that had
        // copied Solemn Simulacrum died without drawing anybody a card.
        let looked_back = if all_kinds {
            None
        } else {
            // Read the departing incarnation before a raw copied list: a
            // back face resets in its new zone, and lost abilities stay lost.
            state
                .ltb_versions
                .iter()
                .find(|(id, _)| *id == permanent)
                .and_then(|(_, version)| {
                    state.source_object(baylee_core::ids::DamageSourceRef {
                        object: permanent,
                        version: *version,
                    })
                })
                .map(|departed| departed.ability_list(lookup))
                .or_else(|| {
                    state
                        .ltb_abilities
                        .iter()
                        .find(|(id, _)| *id == permanent)
                        .map(|(_, list)| list.clone())
                })
        };
        // Not `obj.card`: a token has none, and bailing out here is how every
        // token on the battlefield used to be invisible to triggers —
        // including its own.
        let list = looked_back.unwrap_or_else(|| obj.ability_list(lookup));
        let abilities = list.abilities.clone();
        // Prowess (engine-level keyword trigger, CR 702.108).
        if obj
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::PROWESS)
        {
            for entry in events {
                if let GameEvent::SpellCast { object, player, .. } = &entry.event
                    && *player == obj.controller
                    && state.object(*object).is_some_and(|spell| {
                        !spell
                            .characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                    })
                {
                    for _ in 0..times_triggered(
                        state,
                        baylee_cards_dsl::TriggerEventKind::Any,
                        permanent,
                    ) {
                        let source = source_at_event(state, obj, events, entry, false);
                        triggers.push(PendingTrigger {
                            text: TextChangeMap::IDENTITY,
                            source_version: Some(source.version),
                            related_objects: None,
                            event_subject: None,
                            event_ability: None,
                            event_object_identity: None,
                            counter_source_version: None,
                            event_mana: None,
                            event_mana_value: None,
                            event_departure: None,
                            event_amount: None,
                            event_player: None,
                            source: permanent,
                            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                            controller: source.controller,
                            timestamp: source.timestamp,
                            event_object: Some(permanent),
                            implicit_target: Some(permanent),
                            abilities: None,
                            synthetic_effects: Some(PROWESS_PUMP),
                            once_per_turn: false,
                            synthetic_target: None,
                            chosen_mode: None,
                        });
                    }
                    // Once per spell, not once per window: two noncreature
                    // spells can land in one of these (a spell cast during
                    // another's resolution), and prowess counts both.
                }
            }
        }
        if !all_kinds {
            return_keywords::collect(state, obj, events, triggers);
        }
        // Granted triggered abilities (class levels): continuous effects
        // carrying GrantTriggered that apply to this permanent.
        for &fx in &granted {
            let baylee_cards_dsl::Modifier::GrantTriggered {
                trigger,
                effects,
                target,
            } = &fx.modifier
            else {
                continue;
            };
            let text = state.effect_text(fx);
            let context = RuleContext {
                source: permanent,
                text,
                reference: None,
            };
            let look_back = matches!(
                trigger,
                Trigger::Dies(_)
                    | Trigger::DiesNotSacrificed(_)
                    | Trigger::LeavesBattlefield(_)
                    | Trigger::LinkedCreatureLeaves
            );
            if !all_kinds && !look_back {
                continue;
            }
            for entry in events {
                let source = source_at_event(state, obj, events, entry, look_back);
                // A grant belongs to its recipient's battlefield incarnation,
                // not the new card in the graveyard (CR 603.10a).
                let applies = if look_back {
                    crate::effects::applies_to_snapshot(state, fx, source)
                } else {
                    crate::effects::applies_to(state, fx, source)
                };
                if !applies {
                    continue;
                }
                let hit = departure_hit(trigger, entry, permanent).unwrap_or_else(|| {
                    hits_with_context(
                        trigger,
                        &entry.event,
                        events,
                        state,
                        context,
                        source.controller,
                    )
                });
                if hit > 0 {
                    let event_object = event_object_for(trigger, &entry.event, permanent, state);
                    let times = trigger_count(state, trigger, permanent, source.controller) * hit;
                    for _ in 0..times {
                        triggers.push(PendingTrigger {
                            text,
                            source_version: Some(source.version),
                            related_objects: None,
                            event_subject: event_subject_for(
                                trigger,
                                &entry.event,
                                permanent,
                                state,
                            ),
                            event_ability: activated_stack_ability(&entry.event),
                            event_object_identity: event_object
                                .and_then(|id| state.event_object_identity(id, entry.seq)),
                            counter_source_version: None,
                            event_mana: None,
                            event_mana_value: cast_mana_value(state, &entry.event),
                            event_departure: entry
                                .departure
                                .as_ref()
                                .map(|d| (d.controller, d.toughness)),
                            event_amount: None,
                            event_player: event_player_of(&entry.event),
                            source: permanent,
                            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                            controller: source.controller,
                            timestamp: source.timestamp,
                            event_object,
                            implicit_target: matches!(trigger, Trigger::Ward)
                                .then_some(event_object)
                                .flatten(),
                            abilities: None,
                            synthetic_effects: Some(effects),
                            synthetic_target: *target,
                            once_per_turn: false,
                            chosen_mode: None,
                        });
                    }
                    // Once per matching event, as below.
                }
            }
        }
        // Ward {N} (engine-level keyword trigger, CR 702.21): an
        // opponent's spell or ability targets this permanent.
        for ability in &abilities {
            let AbilityDef::Ward { mana } = ability else {
                continue;
            };
            let Some(synthetic) = WARD_PAY_OR_COUNTER
                .get(usize::from(*mana))
                .map(|effects| &effects[..])
            else {
                continue; // a ward cost past the table's ceiling
            };
            for entry in events {
                let Some((target_obj, caster)) = targeting(&entry.event, state, permanent) else {
                    continue;
                };
                if state.is_opponent(caster, obj.controller) {
                    for _ in 0..times_triggered(
                        state,
                        baylee_cards_dsl::TriggerEventKind::Any,
                        permanent,
                    ) {
                        let source = source_at_event(state, obj, events, entry, false);
                        triggers.push(PendingTrigger {
                            text: TextChangeMap::IDENTITY,
                            source_version: Some(source.version),
                            related_objects: None,
                            event_subject: None,
                            event_ability: None,
                            event_object_identity: None,
                            counter_source_version: None,
                            event_mana: None,
                            event_mana_value: None,
                            event_departure: None,
                            event_amount: None,
                            event_player: None,
                            source: permanent,
                            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                            controller: source.controller,
                            timestamp: source.timestamp,
                            event_object: Some(target_obj),
                            implicit_target: Some(target_obj),
                            abilities: None,
                            synthetic_effects: Some(synthetic),
                            once_per_turn: false,
                            synthetic_target: None,
                            chosen_mode: None,
                        });
                    }
                }
            }
        }
        for (index, ability) in abilities.iter().enumerate() {
            let Some(firing) = triggered_parts(ability) else {
                continue;
            };
            if firing.zone != TriggerZone::Battlefield {
                continue;
            }
            let trigger = firing.trigger;
            // Off the battlefield, the triggers that look back (CR 603.10a)
            // and "when you cycle this card", which triggers from wherever
            // the card winds up (CR 702.29c).
            if !all_kinds
                && !matches!(
                    trigger,
                    Trigger::LeavesBattlefield(_)
                        | Trigger::LinkedCreatureLeaves
                        | Trigger::Dies(_)
                        | Trigger::DiesNotSacrificed(_)
                        | Trigger::CycledThis
                )
            {
                continue;
            }
            // CR 603.4, the first of its two checks: an ability whose
            // intervening-`if` clause is false does not trigger at all.
            // Before `trigger_count`, so a trigger multiplier has nothing
            // to double — Panharmonicon doubles a trigger, not a
            // non-trigger.
            for entry in events {
                let source = source_at_event(
                    state,
                    obj,
                    events,
                    entry,
                    matches!(
                        trigger,
                        Trigger::Dies(_)
                            | Trigger::DiesNotSacrificed(_)
                            | Trigger::LeavesBattlefield(_)
                            | Trigger::LinkedCreatureLeaves
                    ),
                );
                let context = ability_context(state, source, &list, index);
                if !condition_matches(
                    state,
                    firing.condition,
                    source.controller,
                    context,
                    Some(entry),
                ) {
                    continue;
                }
                let linked_hit = (*trigger == Trigger::LinkedCreatureLeaves).then(|| {
                    crate::resolve::linked_reanimation::departure(source, list.acquisition, entry)
                });
                let hit = linked_hit
                    .or_else(|| departure_hit(trigger, entry, permanent))
                    .unwrap_or_else(|| {
                        hits_with_context(
                            trigger,
                            &entry.event,
                            events,
                            state,
                            context,
                            source.controller,
                        )
                    });
                if hit > 0 {
                    let times = trigger_count(state, trigger, permanent, obj.controller) * hit;
                    let event_object = event_object_for(trigger, &entry.event, permanent, state);
                    let event_damage = event_damage_of(trigger, &entry.event, events);
                    for _ in 0..times {
                        let source = source_at_event(
                            state,
                            obj,
                            events,
                            entry,
                            matches!(
                                trigger,
                                Trigger::Dies(_)
                                    | Trigger::DiesNotSacrificed(_)
                                    | Trigger::LeavesBattlefield(_)
                                    | Trigger::LinkedCreatureLeaves
                            ),
                        );
                        triggers.push(PendingTrigger {
                            text: TextChangeMap::IDENTITY,
                            source_version: Some(source.version),
                            related_objects: None,
                            event_subject: event_subject_for(
                                trigger,
                                &entry.event,
                                permanent,
                                state,
                            ),
                            event_ability: activated_stack_ability(&entry.event),
                            event_object_identity: event_object
                                .and_then(|id| state.event_object_identity(id, entry.seq)),
                            counter_source_version: crate::resolve::linked_counters::uses_links(
                                &abilities,
                            )
                            .then(|| {
                                // A source may return during the resolution that killed it,
                                // before this event batch is collected.
                                if matches!(entry.event, GameEvent::ZoneChanged {
                                    object, from: Zone::Battlefield, to: Zone::Graveyard, ..
                                } if object == permanent)
                                {
                                    state
                                        .ltb_versions
                                        .iter()
                                        .find(|(id, _)| *id == permanent)
                                        .map_or(obj.version, |(_, version)| *version)
                                } else {
                                    obj.version
                                }
                            }),
                            event_mana: produced_mana(&entry.event, events),
                            event_mana_value: cast_mana_value(state, &entry.event),
                            event_departure: entry
                                .departure
                                .as_ref()
                                .map(|d| (d.controller, d.toughness)),
                            event_amount: event_damage,
                            event_player: event_player_of(&entry.event),
                            source: permanent,
                            ability_index: index as u32,
                            abilities: Some(list.clone()),
                            controller: source.controller,
                            timestamp: source.timestamp,
                            event_object,
                            implicit_target: matches!(trigger, Trigger::Ward)
                                .then_some(event_object)
                                .flatten(),
                            synthetic_effects: None,
                            once_per_turn: firing.once_per_turn,
                            synthetic_target: None,
                            chosen_mode: None,
                        });
                    }
                    // No `break`. An ability triggers once per event that
                    // matches it, and there used to be one here, so a
                    // window carrying six of them fired it once: Aang and
                    // Katara made six Allies, Wartime Protestors' rally
                    // gave a counter and haste to the first one and the
                    // other five entered unnoticed. Ondu Cleric gained one
                    // life for six Allies for the same reason.
                    //
                    // The carve-out that made the old shape look right is
                    // an ability worded "whenever one or more …", which
                    // does fire once for a whole batch — Storm the Vault.
                    // That is a trigger of its own and not the default. The
                    // transcoder refuses the reference's batch modes
                    // outright, and Storm the Vault, which is hand-written,
                    // is `Coverage::Partial` saying it makes a Treasure per
                    // creature, so nothing relies on the accident. Giving it
                    // a `Trigger` variant is what the card will want, not
                    // this line.
                }
            }
        }
    }
}

/// How often a trigger fires: trigger multipliers (Panharmonicon) add,
/// suppressors (Elesh Norn) zero it out.
/// How many times the thing a journal entry records actually happened.
///
/// A journal entry is one happening and this returns 1 for all but one of
/// them. `GameEvent::CardsDrawn` is the exception: `GameState::draw_cards`
/// moves the cards one at a time and then records a *single* entry carrying
/// the count, and a player draws cards one at a time (so "draw two cards" is
/// two draws), which makes "whenever you draw a card" fire twice for it.
///
/// This is entry 35's defect in the one shape its fix could not see. That one
/// was a `break` firing an ability once for a whole *list* of matching
/// events; this is a batch that is a field, and no amount of not-breaking
/// finds it. The knowledge lives here rather than in the three collection
/// loops so that the next event to carry a count adds an arm to one `match`
/// instead of a multiplication to each of them.
///
/// [`hits`] asks it, so a trigger that watches all of an entry's happenings
/// but some can take them away. The number is multiplied with
/// [`trigger_count`], not confused with it:
/// that one is Panharmonicon asking how many times an ability triggers for
/// one happening, this one is how many happenings there were.
fn repeats(event: &GameEvent) -> u32 {
    match event {
        // `.max(1)` is belt and braces, not a live case: `draw_cards`
        // records inside `if !drawn.is_empty()`, so a count of zero is
        // unreachable today. It is here because the failure it guards
        // against is silent in the wrong direction — a zero would
        // *suppress* a trigger that matched rather than over-fire it.
        GameEvent::CardsDrawn { count, .. } => u32::from(*count).max(1),
        _ => 1,
    }
}

fn trigger_count(
    state: &GameState,
    trigger: &Trigger,
    source: ObjectId,
    controller: PlayerId,
) -> u32 {
    let event_kind = match trigger {
        Trigger::EntersBattlefield(_) | Trigger::EnteredThisFrom(_) => {
            baylee_cards_dsl::TriggerEventKind::EntersBattlefield
        }
        _ => baylee_cards_dsl::TriggerEventKind::Any,
    };
    let _ = controller;
    times_triggered(state, event_kind, source)
}

/// How many times a triggered ability of `source` triggers for one event
/// of `event_kind` (CR 603.2d): once, plus one for each multiplier whose
/// filter the source matches, or not at all under a suppressor.
///
/// Every triggered ability an object has asks this, the printed ones
/// through [`trigger_count`] and the keyword ones directly. Prowess is a
/// triggered ability (CR 702.108a), ward is one (CR 702.21a), undying
/// (CR 702.93a) and persist (CR 702.79a) are, and so is a granted trigger; Katara, the Fearless multiplies "a triggered
/// ability of an Ally you control", which is all of them. The keyword
/// pushes used to build one trigger each and never ask, so Katara doubled
/// Sokka's token and left his prowess, and every Ally's he granted, at one
/// (#318).
pub(crate) fn times_triggered(
    state: &GameState,
    event_kind: baylee_cards_dsl::TriggerEventKind,
    source: ObjectId,
) -> u32 {
    let Some(source_obj) = state.object(source) else {
        return 1;
    };
    times_triggered_with_source(state, event_kind, source_obj)
}

fn times_triggered_with_source(
    state: &GameState,
    event_kind: baylee_cards_dsl::TriggerEventKind,
    source_obj: &crate::object::GameObject,
) -> u32 {
    let mut count = 1u32;
    for entry in &state.replacement_rules {
        match entry.rule {
            baylee_cards_dsl::ReplacementRule::TriggerMultiplier {
                source_filter,
                event,
            } if (event == event_kind || event == baylee_cards_dsl::TriggerEventKind::Any)
                && eval::matches(
                    source_filter,
                    state,
                    source_obj,
                    entry.controller,
                    entry.source,
                ) =>
            {
                #[cfg(test)]
                crate::ability_log::replaced(entry);
                count += 1;
            }
            baylee_cards_dsl::ReplacementRule::TriggerSuppress {
                source_filter,
                event,
            } if (event == event_kind || event == baylee_cards_dsl::TriggerEventKind::Any)
                && eval::matches(
                    source_filter,
                    state,
                    source_obj,
                    entry.controller,
                    entry.source,
                ) =>
            {
                #[cfg(test)]
                crate::ability_log::replaced(entry);
                return 0;
            }
            _ => {}
        }
    }
    count
}

#[allow(clippy::too_many_lines)] // the trigger×event matrix is naturally one flat table
fn matches(
    trigger: &Trigger,
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
    state: &GameState,
    context: RuleContext,
    you: PlayerId,
) -> bool {
    let source = context.source;
    match (trigger, event) {
        // CR 701.27e for the second: the ability is read off the face the
        // permanent shows right after it turned over, which is the face that
        // prints it. CR 702.29c for the third: the card cycled is the
        // source, wherever it wound up.
        (Trigger::TurnedFaceUp, GameEvent::TurnedFaceUp { object })
        | (Trigger::TransformsIntoThis, GameEvent::Transformed { object, .. })
        | (Trigger::CycledThis, GameEvent::Cycled { object, .. })
        | (Trigger::Rampage, GameEvent::AttackerBlocked { object }) => *object == source,
        // CR 709.5h: the designation, however it was given.
        (Trigger::UnlockThisDoor(door), GameEvent::DoorUnlocked { object, half }) => {
            *object == source && door == half
        }
        (
            Trigger::EntersBattlefield(filter),
            GameEvent::ZoneChanged {
                object,
                to: Zone::Battlefield,
                ..
            },
        ) => state
            .object(*object)
            .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context)),
        (
            Trigger::EnteredThisFrom(origin),
            GameEvent::ZoneChanged {
                object,
                from,
                to: Zone::Battlefield,
                ..
            },
        ) => {
            *object == source
                && match origin {
                    baylee_cards_dsl::ZoneRef::Battlefield => *from == Zone::Battlefield,
                    baylee_cards_dsl::ZoneRef::Graveyard => *from == Zone::Graveyard,
                    baylee_cards_dsl::ZoneRef::Hand => *from == Zone::Hand,
                    baylee_cards_dsl::ZoneRef::Library => *from == Zone::Library,
                    baylee_cards_dsl::ZoneRef::Exile => *from == Zone::Exile,
                    baylee_cards_dsl::ZoneRef::Stack => *from == Zone::Stack,
                    baylee_cards_dsl::ZoneRef::Command => *from == Zone::Command,
                    baylee_cards_dsl::ZoneRef::NotBattlefield => *from != Zone::Battlefield,
                }
        }
        // The three leaves-the-battlefield triggers look back (CR 603.10a).
        (
            Trigger::LeavesBattlefield(filter),
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                ..
            },
        )
        | (
            Trigger::ExiledFromBattlefield(filter),
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                to: Zone::Exile,
                ..
            },
        )
        | (
            Trigger::Dies(filter),
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                to: Zone::Graveyard,
                ..
            },
        ) => departed_matches(filter, state, *object, you, context),
        (
            Trigger::DiesNotSacrificed(filter),
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                to: Zone::Graveyard,
                ..
            },
        ) => {
            event_groups::index_of_event(event, batch)
                .and_then(|index| batch.get(index))
                .is_some_and(|entry| {
                    entry
                        .departure
                        .as_ref()
                        .is_some_and(|departure| !departure.sacrificed)
                })
                && departed_matches(filter, state, *object, you, context)
        }
        (
            Trigger::OneOrMoreDealCombatDamageToPlayer(filter),
            GameEvent::DamageDealt {
                target: crate::event::DamageTarget::Player(_),
                is_combat: true,
                amount: 1..,
                ..
            },
        ) => event_groups::first_matching_damage(event, batch, state, you, context, filter),
        (
            Trigger::DealsCombatDamageToPlayer(filter),
            GameEvent::DamageDealt {
                source: Some(damage_source),
                target: crate::event::DamageTarget::Player(_),
                is_combat: true,
                ..
            },
        ) => state
            .object(*damage_source)
            .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context)),
        // Contested War Zone: each source's own combat damage to the
        // ability's controller; the source is the event object.
        (
            Trigger::DealsCombatDamageToYou(filter),
            GameEvent::DamageDealt {
                source: Some(damage_source),
                target: crate::event::DamageTarget::Player(player),
                is_combat: true,
                amount,
            },
        ) => {
            *player == you
                && *amount > 0
                && state
                    .object(*damage_source)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        (
            Trigger::DealsCombatDamageToOpponent(filter),
            GameEvent::DamageDealt {
                source: Some(damage_source),
                target: crate::event::DamageTarget::Player(player),
                is_combat: true,
                ..
            },
        ) => {
            state.is_opponent(*player, you)
                && state
                    .object(*damage_source)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        (
            Trigger::DealsDamageToPlayer {
                source: filter,
                player: who,
            },
            GameEvent::DamageDealt {
                source: Some(damage_source),
                target: crate::event::DamageTarget::Player(player),
                amount,
                ..
            },
        ) => {
            *amount > 0
                && eval::players(*who, state, you).is_some_and(|players| players.contains(player))
                && state.object(*damage_source).is_some_and(|object| {
                    eval::matches_with_context(filter, state, object, you, context)
                })
        }
        (
            Trigger::DealsDamageToOpponent(filter),
            GameEvent::DamageDealt {
                source: Some(damage_source),
                target: crate::event::DamageTarget::Player(player),
                amount,
                ..
            },
        ) => {
            *amount > 0
                && state.is_opponent(*player, you)
                && state
                    .object(*damage_source)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        // "Whenever [this] deals damage" (El-Hajjâj): every damage event
        // the source deals, to anything, and a step's combat damage once
        // (CR 510.2, 603.2c).
        (
            Trigger::DealsDamage(filter),
            GameEvent::DamageDealt {
                source: Some(damage_source),
                amount,
                is_combat,
                ..
            },
        ) => {
            *amount > 0
                && (!*is_combat || first_combat_damage_by(event, batch, *damage_source))
                && state
                    .object(*damage_source)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        (
            Trigger::DealtDamage(filter),
            GameEvent::DamageDealt {
                target: crate::event::DamageTarget::Object(dealt),
                amount,
                is_combat,
                ..
            },
        ) => {
            *amount > 0
                && (!*is_combat
                    || first_combat_damage_to(
                        event,
                        batch,
                        crate::event::DamageTarget::Object(*dealt),
                    ))
                && state
                    .object(*dealt)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        // The player's side of the arm above: once for a step's combat
        // damage to that player, once for every other damage event.
        (
            Trigger::PlayerDealtDamage(rel),
            GameEvent::DamageDealt {
                target: target @ crate::event::DamageTarget::Player(player),
                amount,
                is_combat,
                ..
            },
        ) => {
            *amount > 0
                && eval::players(*rel, state, you).is_some_and(|seats| seats.contains(player))
                && (!*is_combat || first_combat_damage_to(event, batch, *target))
        }
        (
            Trigger::CountersPlaced {
                filter,
                kind,
                by,
                first_this_turn,
            },
            GameEvent::CounterChanged {
                object,
                version,
                kind: changed,
                old,
                new,
                by: Some(placer),
                first_this_turn: first,
            },
        ) => {
            new > old
                && changed == kind
                && (!first_this_turn || *first)
                && eval::players(*by, state, you).is_some_and(|players| players.contains(placer))
                && state
                    .source_object(baylee_core::ids::DamageSourceRef {
                        object: *object,
                        version: *version,
                    })
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        // CR 714.2b's window, "was less than N and became at least N",
        // asked of the source's own counters.
        (
            Trigger::CountersReach { kind, n },
            GameEvent::CounterChanged {
                object,
                kind: changed,
                old,
                new,
                ..
            },
        ) => *object == source && changed == kind && *old < u16::from(*n) && u16::from(*n) <= *new,
        // "When you remove the last [kind] counter from this": an effect of
        // the ability's controller took the count from some to none.
        (
            Trigger::LastCounterRemoved { kind },
            GameEvent::CounterChanged {
                object,
                kind: changed,
                old,
                new,
                by: Some(remover),
                ..
            },
        ) => *object == source && changed == kind && *old > 0 && *new == 0 && *remover == you,
        (
            Trigger::TappedForMana { by, filter },
            GameEvent::ManaProduced {
                player,
                source: Some(tapped),
                ..
            },
        ) => {
            // Who tapped it: the mana is the activating player's, and the
            // relation is asked of the state alone (`You`, `EachPlayer`,
            // `EachOpponent`; a relation that needs a resolution names
            // nobody here).
            eval::players(*by, state, you).is_some_and(|seats| seats.contains(player))
                && first_mana_of_a_tap(event, state.journal.entries(), *tapped)
                && state
                    .object(*tapped)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        // Any permanent the filter matches: City of Brass's `Filter::This`
        // is its source, Lifetap's is a Forest an opponent controls, and
        // Psychic Venom's the land it enchants.
        (
            Trigger::AbilityActivated {
                filter,
                who,
                tap_cost,
                mana_ability,
            },
            GameEvent::AbilityActivated {
                source: activated,
                player,
                tap_cost: tapped,
                mana_ability: mana,
                ..
            },
        ) => {
            tap_cost.is_none_or(|required| required == *tapped)
                && mana_ability.is_none_or(|required| required == *mana)
                && eval::players(*who, state, you).is_some_and(|players| players.contains(player))
                && state.source_object(*activated).is_some_and(|object| {
                    activations::source_matches(filter, state, object, you, context)
                })
        }
        (
            Trigger::DealsCombatDamageToCreature(filter),
            GameEvent::DamageDealt {
                source: Some(dealer),
                target: crate::event::DamageTarget::Object(recipient),
                amount,
                is_combat: true,
            },
        ) => {
            *amount > 0
                && state.object(*dealer).is_some_and(|object| {
                    eval::matches_with_context(filter, state, object, you, context)
                })
                && state.object(*recipient).is_some_and(|object| {
                    object
                        .characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
                })
        }
        (Trigger::BecomesTapped(filter), GameEvent::ObjectTapped { object, .. }) => state
            .object(*object)
            .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context)),
        (
            Trigger::SpellCast(filter) | Trigger::SpellCastOrCopied(filter),
            GameEvent::SpellCast { object, .. },
        )
        | (Trigger::SpellCastOrCopied(filter), GameEvent::SpellCopied { object, .. }) => state
            .object(*object)
            .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context)),
        (
            Trigger::InstantCastAfterFirst { who },
            GameEvent::SpellCast {
                player,
                instant_number,
                ..
            },
        ) => {
            *instant_number > 1
                && match who {
                    PlayerRel::Opponent | PlayerRel::EachOpponent => {
                        state.is_opponent(*player, you)
                    }
                    _ => {
                        eval::players(*who, state, you).is_some_and(|seats| seats.contains(player))
                    }
                }
        }
        (Trigger::NthSpellCast { n, filter }, GameEvent::SpellCast { object, number, .. }) => {
            *number == u32::from(*n)
                && state
                    .object(*object)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        (Trigger::SpellCastUsingSourceMana(filter), GameEvent::SpellCast { object, .. }) => {
            state.object(*object).is_some_and(|spell| {
                eval::matches_with_context(filter, state, spell, you, context)
                    && spell.paid.as_ref().is_some_and(|paid| {
                        let version = state.object(source).map(|object| object.version);
                        paid.mana_paid
                            .restricted()
                            .iter()
                            .chain(paid.mana_paid.ridden())
                            .any(|mana| {
                                mana.amount > 0
                                    && state.restriction_info.get(&mana.restriction.0).is_some_and(
                                        |provenance| {
                                            provenance.context.source == source
                                                && provenance.source_version == version
                                        },
                                    )
                            })
                    })
            })
        }
        (
            Trigger::Discarded { who, filter },
            GameEvent::Discarded {
                object,
                version,
                player,
                known: true,
                ..
            },
        ) => {
            eval::players(*who, state, you).is_some_and(|players| players.contains(player))
                && state
                    .source_object(baylee_core::ids::DamageSourceRef {
                        object: *object,
                        version: *version,
                    })
                    .is_some_and(|card| {
                        eval::matches_with_context(filter, state, card, you, context)
                    })
        }
        (Trigger::BecomesTarget, _) => targeting(event, state, source).is_some(),
        (Trigger::Ward, _) => targeting(event, state, source)
            .is_some_and(|(_, controller)| state.is_opponent(controller, you)),
        (
            Trigger::EntersBattlefieldEvoked,
            GameEvent::ZoneChanged {
                object,
                to: Zone::Battlefield,
                ..
            },
        ) => *object == source && state.object(*object).is_some_and(|o| o.alt_cast),
        (Trigger::PlaysAnotherLand, GameEvent::LandPlayed { player, object }) => {
            *player == you && *object != source
        }
        (Trigger::Draws(rel), GameEvent::CardsDrawn { player, .. })
        | (Trigger::PlaysLand(rel), GameEvent::LandPlayed { player, .. }) => match rel {
            PlayerRel::You => *player == you,
            PlayerRel::Opponent | PlayerRel::EachOpponent => state.is_opponent(*player, you),
            _ => true,
        },
        (Trigger::AttackWithAtLeast(minimum), GameEvent::BecameAttacker { .. }) => {
            event_groups::attack_threshold(event, batch, state, you, *minimum, None)
        }
        (Trigger::AttackWithAtLeastMatching { n, filter }, GameEvent::BecameAttacker { .. }) => {
            event_groups::attack_threshold(event, batch, state, you, *n, Some((filter, context)))
        }
        (Trigger::Attacks(filter), GameEvent::BecameAttacker { object, .. }) => state
            .object(*object)
            .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context)),
        // "Whenever one or more … attack": on the first matching attacker of
        // the declaration and on no other (CR 603.2c), which is one batch.
        (Trigger::OneOrMoreAttack(filter), GameEvent::BecameAttacker { object, .. }) => {
            let hit = |id: ObjectId| {
                state
                    .object(id)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
            };
            let Some(at) = event_groups::index_of_event(event, batch) else {
                return false;
            };
            hit(*object)
                && !batch[..at].iter().any(|entry| {
                    matches!(entry.event, GameEvent::BecameAttacker { object, .. } if hit(object))
                })
        }
        // CR 509.3b and 509.3d: one event per blocker–attacker pair, so the
        // ability triggers once for each creature this one blocks and once
        // for each creature that blocks it. The filter is asked of the other
        // creature as it is now, when it has just blocked or been blocked
        // (CR 509.3f).
        (
            Trigger::BlocksOrBecomesBlockedBy(filter),
            GameEvent::BecameBlocker {
                object: blocker,
                attacker,
            },
        ) => {
            let other = if *blocker == source {
                *attacker
            } else if *attacker == source {
                *blocker
            } else {
                return false;
            };
            state
                .object(other)
                .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        // The same pair, seen from the creature this source is attached to
        // (Infinite Authority's "enchanted creature"): the source's own
        // ability (CR 603.3a), once per pair.
        (
            Trigger::AttachedBlocksOrBecomesBlockedBy(filter),
            GameEvent::BecameBlocker {
                object: blocker,
                attacker,
            },
        ) => {
            let Some(host) = state.object(source).and_then(|o| o.attached_to) else {
                return false;
            };
            let other = if *blocker == host {
                *attacker
            } else if *attacker == host {
                *blocker
            } else {
                return false;
            };
            state
                .object(other)
                .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        // CR 509.3d: one event per blocker of this creature.
        (
            Trigger::BecomesBlockedBy(filter),
            GameEvent::BecameBlocker {
                object: blocker,
                attacker,
            },
        ) => {
            *attacker == source
                && state
                    .object(*blocker)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        // CR 509.3c: once, on the first blocker declared for it.
        (Trigger::BecomesBlocked, GameEvent::BecameBlocker { attacker, .. }) => {
            if *attacker != source {
                return false;
            }
            let Some(at) = batch
                .iter()
                .position(|entry| std::ptr::eq(&raw const entry.event, event))
            else {
                return false;
            };
            !state.combat.blockers_of(source).is_empty()
                && !batch[..at].iter().any(|entry| {
                    matches!(entry.event, GameEvent::BecameBlocker { attacker: earlier, .. } if earlier == source)
                })
        }
        // CR 509.3a: once per blocker, on the first of its pairs in the
        // declaration, however many attackers it blocks.
        (Trigger::BlocksCreature(filter), GameEvent::BecameBlocker { object, attacker }) => {
            *object == source
                && state.object(*attacker).is_some_and(|other| {
                    eval::matches_with_context(filter, state, other, you, context)
                })
        }
        (Trigger::Blocks(filter), GameEvent::BecameBlocker { object, .. }) => {
            let Some(at) = event_groups::index_of_event(event, batch) else {
                return false;
            };
            !batch[..at].iter().any(|entry| {
                matches!(entry.event, GameEvent::BecameBlocker { object: earlier, .. } if earlier == *object)
            }) && state
                .object(*object)
                .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        (Trigger::AttacksAndIsntBlocked(filter), GameEvent::AttackerUnblocked { object }) => state
            .object(*object)
            .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context)),
        (Trigger::AttacksAlone(filter), GameEvent::BecameAttacker { object, .. }) => {
            batch
                .iter()
                .filter(|entry| matches!(entry.event, GameEvent::BecameAttacker { .. }))
                .count()
                == 1
                && state
                    .object(*object)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        (Trigger::FirstNoncreatureSpellCast(rel), GameEvent::SpellCast { object, player, .. }) => {
            let player_matches = match rel {
                PlayerRel::You => *player == you,
                PlayerRel::Opponent | PlayerRel::EachOpponent => state.is_opponent(*player, you),
                _ => true,
            };
            if !player_matches {
                return false;
            }
            let is_noncreature = state.object(*object).is_some_and(|o| {
                !o.characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
            });
            let count = state
                .per_turn
                .noncreature_spells
                .get(player.get() as usize)
                .copied()
                .unwrap_or(0);
            is_noncreature && count == 1
        }
        (
            Trigger::StepBeginChosenOpponent { step },
            GameEvent::StepChanged {
                step: began_step, ..
            },
        ) => {
            begins_step(state, *began_step, *step)
                && state
                    .object(source)
                    .and_then(crate::object::GameObject::chosen_opponent)
                    == Some(state.turn.active)
        }
        (
            Trigger::StepBegin { step, whose },
            GameEvent::StepChanged {
                step: began_step, ..
            },
        ) => {
            if !begins_step(state, *began_step, *step) {
                return false;
            }
            match whose {
                PlayerRel::You => state.turn.active == you,
                PlayerRel::Opponent => state.is_opponent(state.turn.active, you),
                // "At the beginning of the upkeep of enchanted land's
                // controller" (Cursed Land): that controller's step only.
                PlayerRel::ControllerOfAttached => {
                    crate::eval::controller_of_attached(state, source) == Some(state.turn.active)
                }
                _ => true,
            }
        }
        _ => false,
    }
}

impl PendingTrigger {
    pub(crate) fn bind_target(
        &self,
        spec: baylee_cards_dsl::TargetSpec,
    ) -> baylee_cards_dsl::TargetSpec {
        match spec {
            baylee_cards_dsl::TargetSpec::CardInGraveyardBelowEvent(filter, rel) => {
                baylee_cards_dsl::TargetSpec::CardInGraveyardBelowValue(
                    filter,
                    rel,
                    self.event_mana_value.unwrap_or(0),
                )
            }
            // "That player": the one the event was about, for Questing Beast
            // the one it dealt damage to. With no such event it stays
            // unbound and offers nothing.
            baylee_cards_dsl::TargetSpec::ObjectOfEventPlayer(filter) => match self.event_player {
                Some(player) => baylee_cards_dsl::TargetSpec::ObjectControlledBy(filter, player),
                None => spec,
            },
            other => other,
        }
    }
}

fn activated_stack_ability(event: &GameEvent) -> Option<baylee_core::ids::DamageSourceRef> {
    match event {
        GameEvent::AbilityActivated { ability, .. } => *ability,
        _ => None,
    }
}

#[cfg(test)]
mod tests;
