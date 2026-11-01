//! Triggered abilities: collection and APNAP ordering.
//!
//! After every mutation batch the engine scans new journal entries and
//! matches them against the triggered abilities of permanents and cards
//! whose abilities function in other zones. Matches go on the stack in APNAP
//! order. Same-controller ordering currently follows timestamp; the
//! player-facing ordering choice is M2 (documented in engine-internals).

mod activations;
pub(crate) mod battles;
pub(crate) mod bird_entry;
mod discarded;
mod event_groups;
mod events;
mod flanking;
mod matching;
mod radiation;
mod return_keywords;
mod sagas;
mod speed;
mod watches;

pub(crate) use return_keywords::return_keyword_counts;

use crate::eval;
use crate::event::GameEvent;
use crate::state::{CardLookup, GameState};
use crate::text_changes::{RuleContext, TextChangeMap};
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{AbilityDef, Condition, PlayerRel, Trigger, TriggerZone};
use baylee_core::ids::{ObjectId, PlayerId};

use events::{
    event_damage_of, event_object_for, event_player_of, event_subject_for, object_moves,
    source_at_event, source_at_event_with_moves, targeting,
};
pub(crate) use matching::times_triggered;
use matching::{departure_hit, hits_with_context, times_triggered_with_source, trigger_count};
pub use watches::state_triggers;
use watches::{
    attack_watch_triggers, cast_copy_triggers, damage_watch_triggers, monarch_triggers,
    produced_mana, watch_triggers,
};

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
    /// Target requirement for synthetic triggers that need a target choice
    /// (granted triggered abilities, class levels, a reflexive "target").
    /// It is written onto the stack object as its `target_req`, which is
    /// what CR 608.2b re-checks at resolution.
    pub synthetic_target: Option<baylee_cards_dsl::TargetReq>,
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
        resolved_name: None,
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
        state.battlefield_seen(),
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
    battles::collect(state, lookup, events, &mut triggers);
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
    let mut departed: smallvec::SmallVec<[ObjectId; 4]> = events
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
    if departed.is_empty() {
        return;
    }
    departed.sort_unstable();
    departed.dedup();
    collect_for_objects(state, lookup, departed, events, false, triggers);
}

/// Printed graveyard and exile abilities read the card as it exists there,
/// never its old battlefield abilities. A card outside the battlefield has no
/// controller, so its owner controls this trigger (CR 113.8).
fn graveyard_triggers(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    let batch_has_zone_changes = events
        .iter()
        .any(|entry| matches!(entry.event, GameEvent::ZoneChanged { .. }));
    for object in graveyard_observers(state, lookup, events, batch_has_zone_changes) {
        let source = object.id;
        let owner = object.owner;
        let current = state.object_or_departed(source).unwrap_or(object);
        let moves = object_moves(events, batch_has_zone_changes, current.id);
        let list = object.ability_list(lookup);
        for (index, ability) in list.abilities.iter().enumerate() {
            let Some(firing) = triggered_parts(ability) else {
                continue;
            };
            let expected_zone = match firing.zone {
                TriggerZone::Graveyard => Zone::Graveyard,
                TriggerZone::Exile => Zone::Exile,
                TriggerZone::Battlefield => continue,
            };
            let context = ability_context(state, object, &list, index);
            let condition_reads_event = firing.condition.is_some_and(eval::condition_reads_event);
            if !condition_reads_event
                && !condition_matches(state, firing.condition, owner, context, None)
            {
                continue;
            }
            for entry in events {
                let at_event = source_at_event_with_moves(state, current, &moves, entry.seq, false);
                if at_event.zone != expected_zone
                    || (expected_zone == Zone::Exile
                        && at_event.status.contains(crate::object::Status::FACE_DOWN))
                    || at_event.version != object.version
                    || (condition_reads_event
                        && !condition_matches(state, firing.condition, owner, context, Some(entry)))
                {
                    continue;
                }
                let hit =
                    hits_with_context(firing.trigger, &entry.event, events, state, context, owner);
                // These replacements describe triggered abilities of
                // permanents (CR 109.2). A graveyard or exile card is not
                // one, even though its last controller is still stored.
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

fn has_off_battlefield_trigger(
    object: &crate::object::GameObject,
    zone: TriggerZone,
    lookup: &impl CardLookup,
) -> bool {
    object
        .abilities(lookup)
        .iter()
        .any(|ability| triggered_parts(ability).is_some_and(|firing| firing.zone == zone))
}

/// Include graveyard and exile incarnations which left later in this mutation batch.
/// A card moved into the zone after an event must not observe that earlier event.
fn graveyard_observers<'a>(
    state: &'a GameState,
    lookup: &impl CardLookup,
    events: &[crate::event::JournalEntry],
    batch_has_zone_changes: bool,
) -> smallvec::SmallVec<[&'a crate::object::GameObject; 4]> {
    let mut objects: smallvec::SmallVec<[&'a crate::object::GameObject; 4]> = (0..state
        .players
        .len())
        .flat_map(|seat| {
            let p = PlayerId::new(seat as u8);
            state
                .zones
                .list(ZoneLocation::Graveyard(p))
                .iter()
                .filter_map(|id| state.object(*id))
                .filter(|object| {
                    object.is_card()
                        && has_off_battlefield_trigger(object, TriggerZone::Graveyard, lookup)
                })
                .chain(
                    state
                        .zones
                        .list(ZoneLocation::Exile(p))
                        .iter()
                        .filter_map(|id| state.object(*id))
                        .filter(|object| {
                            object.is_card()
                                && !object.status.contains(crate::object::Status::FACE_DOWN)
                                && has_off_battlefield_trigger(object, TriggerZone::Exile, lookup)
                        }),
                )
        })
        .collect();
    if batch_has_zone_changes {
        for entry in events {
            let GameEvent::ZoneChanged {
                object,
                from: from @ (Zone::Graveyard | Zone::Exile),
                ..
            } = entry.event
            else {
                continue;
            };
            let trigger_zone = if from == Zone::Graveyard {
                TriggerZone::Graveyard
            } else {
                TriggerZone::Exile
            };
            let Some(current) = state.object_or_departed(object) else {
                continue;
            };
            let after = source_at_event(state, current, events, entry, false);
            if let Some(before) = state.source_object(baylee_core::ids::DamageSourceRef {
                object,
                version: after.version.wrapping_sub(1),
            }) && before.zone == from
                && before.is_card()
                && (from != Zone::Exile
                    || !before.status.contains(crate::object::Status::FACE_DOWN))
                && has_off_battlefield_trigger(before, trigger_zone, lookup)
            {
                objects.push(before);
            }
        }
    }
    if objects.len() > 1 {
        objects.sort_unstable_by_key(|object| (object.id, object.version));
        objects.dedup_by_key(|object| (object.id, object.version));
    }
    objects
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

/// Scans a set of objects for triggered abilities matching the events.
/// `all_kinds` = every trigger kind (battlefield scan); `false` = only
/// LTB/Dies (off-battlefield scan, CR 603.10).
#[allow(clippy::too_many_lines)]
fn collect_for_objects(
    state: &GameState,
    lookup: &impl CardLookup,
    objects: impl IntoIterator<Item = ObjectId>,
    events: &[crate::event::JournalEntry],
    all_kinds: bool,
    triggers: &mut Vec<PendingTrigger>,
) {
    // Most temporary effects only change stats or keywords. Select grants
    // once per scan, not once for every permanent in a token army.
    let granted: smallvec::SmallVec<[&crate::effects::ContinuousEffect; 4]> =
        if state.effects.may_grant_triggered() {
            state
                .effects
                .iter()
                .filter(|fx| {
                    matches!(
                        fx.modifier,
                        baylee_cards_dsl::Modifier::GrantTriggered { .. }
                            | baylee_cards_dsl::Modifier::GrantTriggeredUpToOne { .. }
                    )
                })
                .collect()
        } else {
            smallvec::SmallVec::new()
        };
    let batch_has_zone_changes = events
        .iter()
        .any(|entry| matches!(entry.event, GameEvent::ZoneChanged { .. }));
    let batch_has_spells_cast = events
        .iter()
        .any(|entry| matches!(entry.event, GameEvent::SpellCast { .. }));
    let batch_has_blocks = events
        .iter()
        .any(|entry| matches!(entry.event, GameEvent::BecameBlocker { .. }));
    for permanent in objects {
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
        let moves = object_moves(events, batch_has_zone_changes, obj.id);
        // Prowess (engine-level keyword trigger, CR 702.108).
        if batch_has_spells_cast
            && obj
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
                    let source = source_at_event_with_moves(state, obj, &moves, entry.seq, false);
                    for _ in 0..times_triggered(
                        state,
                        baylee_cards_dsl::TriggerEventKind::Any,
                        permanent,
                    ) {
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
        flanking::collect(state, obj, events, &moves, batch_has_blocks, triggers);
        if !all_kinds {
            return_keywords::collect(state, obj, events, triggers);
        }
        // Granted triggered abilities (class levels): continuous effects
        // carrying GrantTriggered that apply to this permanent.
        for &fx in &granted {
            let (trigger, effects, target) = match &fx.modifier {
                baylee_cards_dsl::Modifier::GrantTriggered {
                    trigger,
                    effects,
                    target,
                } => (
                    trigger,
                    *effects,
                    target.map(|spec| match spec {
                        baylee_cards_dsl::TargetSpec::ObjectOfEachOpponent(_) => {
                            baylee_cards_dsl::TargetReq::up_to(spec, u8::MAX)
                        }
                        _ => baylee_cards_dsl::TargetReq::one(spec),
                    }),
                ),
                baylee_cards_dsl::Modifier::GrantTriggeredUpToOne {
                    trigger,
                    effects,
                    target,
                } => (
                    trigger,
                    *effects,
                    Some(baylee_cards_dsl::TargetReq::up_to_one(*target)),
                ),
                _ => continue,
            };
            let text = state.effect_text(fx);
            let context = RuleContext {
                source: permanent,
                text,
                resolved_name: None,
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
                let source = source_at_event_with_moves(state, obj, &moves, entry.seq, look_back);
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
                    let event_subject = event_subject_for(trigger, &entry.event, permanent, state);
                    let event_ability = activated_stack_ability(&entry.event);
                    let event_object_identity =
                        event_object.and_then(|id| state.event_object_identity(id, entry.seq));
                    let event_mana_value = cast_mana_value(state, &entry.event);
                    let event_departure = entry
                        .departure
                        .as_ref()
                        .map(|d| (d.controller, d.toughness));
                    let event_player = event_player_of(&entry.event);
                    for _ in 0..times {
                        triggers.push(PendingTrigger {
                            text,
                            source_version: Some(source.version),
                            related_objects: None,
                            event_subject,
                            event_ability,
                            event_object_identity,
                            counter_source_version: None,
                            event_mana: None,
                            event_mana_value,
                            event_departure,
                            event_amount: event_damage_of(trigger, &entry.event, events),
                            event_player,
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
                            synthetic_target: target,
                            once_per_turn: false,
                            chosen_mode: None,
                        });
                    }
                    // Once per matching event, as below.
                }
            }
        }
        if all_kinds
            && !obj.abilities(lookup).iter().any(|ability| {
                matches!(ability, AbilityDef::Ward { .. })
                    || triggered_parts(ability)
                        .is_some_and(|firing| firing.zone == TriggerZone::Battlefield)
            })
        {
            continue;
        }
        let current_text = object_text(state, obj);
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
        // Ward {N} (engine-level keyword trigger, CR 702.21): an
        // opponent's spell or ability targets this permanent.
        for ability in &list.abilities {
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
                    let source = source_at_event_with_moves(state, obj, &moves, entry.seq, false);
                    for _ in 0..times_triggered(
                        state,
                        baylee_cards_dsl::TriggerEventKind::Any,
                        permanent,
                    ) {
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
        let uses_counter_links = crate::resolve::linked_counters::uses_links(&list.abilities);
        for (index, ability) in list.abilities.iter().enumerate() {
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
            let look_back = matches!(
                trigger,
                Trigger::Dies(_)
                    | Trigger::DiesNotSacrificed(_)
                    | Trigger::LeavesBattlefield(_)
                    | Trigger::LinkedCreatureLeaves
            );
            let source_is_constant =
                moves.is_empty() && (!look_back || obj.zone == Zone::Battlefield);
            let obj_context = RuleContext {
                source: obj.id,
                text: list.base_text(index).then(current_text),
                resolved_name: None,
                reference: None,
            };
            let condition_reads_event = firing.condition.is_some_and(eval::condition_reads_event);
            let obj_condition_holds = !condition_reads_event
                && condition_matches(state, firing.condition, obj.controller, obj_context, None);
            if source_is_constant && !condition_reads_event && !obj_condition_holds {
                continue;
            }
            // CR 603.4, the first of its two checks: an ability whose
            // intervening-`if` clause is false does not trigger at all.
            // Before `trigger_count`, so a trigger multiplier has nothing
            // to double — Panharmonicon doubles a trigger, not a
            // non-trigger.
            for entry in events {
                let source = if source_is_constant {
                    obj
                } else {
                    source_at_event_with_moves(state, obj, &moves, entry.seq, look_back)
                };
                let same_as_obj = std::ptr::eq(source, obj);
                let context = if same_as_obj {
                    obj_context
                } else {
                    ability_context(state, source, &list, index)
                };
                let cond_ok = if same_as_obj && !condition_reads_event {
                    obj_condition_holds
                } else {
                    condition_matches(
                        state,
                        firing.condition,
                        source.controller,
                        context,
                        Some(entry),
                    )
                };
                if !cond_ok {
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
                    let event_subject = event_subject_for(trigger, &entry.event, permanent, state);
                    let event_ability = activated_stack_ability(&entry.event);
                    let event_object_identity =
                        event_object.and_then(|id| state.event_object_identity(id, entry.seq));
                    let counter_source_version = uses_counter_links.then(|| {
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
                    });
                    let event_mana = produced_mana(&entry.event, events);
                    let event_mana_value = cast_mana_value(state, &entry.event);
                    let event_departure = entry
                        .departure
                        .as_ref()
                        .map(|d| (d.controller, d.toughness));
                    let event_player = event_player_of(&entry.event);
                    for _ in 0..times {
                        triggers.push(PendingTrigger {
                            text: TextChangeMap::IDENTITY,
                            source_version: Some(source.version),
                            related_objects: None,
                            event_subject,
                            event_ability,
                            event_object_identity,
                            counter_source_version,
                            event_mana,
                            event_mana_value,
                            event_departure,
                            event_amount: event_damage,
                            event_player,
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
            baylee_cards_dsl::TargetSpec::CardInGraveyard(
                filter,
                PlayerRel::EventPlayer | PlayerRel::DamagedPlayer,
            ) => match self.event_player {
                Some(player) => {
                    baylee_cards_dsl::TargetSpec::CardInPlayersGraveyard(filter, player)
                }
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

#[cfg(test)]
mod life_loss_tests;
