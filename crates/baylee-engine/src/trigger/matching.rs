//! Whether an event matches a trigger, and how many times it fires.

use super::watches::{first_combat_damage_by, first_combat_damage_to, first_mana_of_a_tap};
use super::{activations, begins_step, event_groups, targeting};
use crate::eval;
use crate::event::GameEvent;
use crate::state::GameState;
use crate::text_changes::RuleContext;
use crate::zone::Zone;
use baylee_cards_dsl::{PlayerRel, Trigger};
use baylee_core::ids::{ObjectId, PlayerId};

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
pub(super) fn hits(
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

pub(super) fn hits_with_context(
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
        Trigger::NthCardDrawnInTurn { who, n } => match event {
            GameEvent::CardsDrawn {
                player,
                count,
                drawn_this_turn_after,
                ..
            } if *count > 0
                && eval::players(*who, state, you).is_some_and(|seats| seats.contains(player)) =>
            {
                let first = drawn_this_turn_after.saturating_sub(u32::from(*count)) + 1;
                u32::from((first..=*drawn_this_turn_after).contains(&u32::from(*n)))
            }
            _ => 0,
        },
        _ => u32::from(matches(trigger, event, batch, state, context, you)) * repeats(event),
    }
}

pub(super) fn departure_hit(
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
            let player = counts_you
                && (obj.target_players.contains(you)
                    || obj.second_target_players().contains(you)
                    || obj.chosen_player == Some(you));
            u32::try_from(objects).unwrap_or(u32::MAX) + u32::from(player)
        }
        _ => 0,
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
pub(super) fn repeats(event: &GameEvent) -> u32 {
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

pub(super) fn trigger_count(
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

pub(super) fn times_triggered_with_source(
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
        | (Trigger::Rampage, GameEvent::AttackerBlocked { object })
        | (Trigger::Mutates, GameEvent::CreatureMutated { object }) => *object == source,
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
        (
            Trigger::DealtCombatDamage(filter),
            GameEvent::DamageDealt {
                target: crate::event::DamageTarget::Object(dealt),
                amount,
                is_combat: true,
                ..
            },
        ) => {
            *amount > 0
                && first_combat_damage_to(event, batch, crate::event::DamageTarget::Object(*dealt))
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
        (Trigger::BecomesTapped(filter), GameEvent::ObjectTapped { object, .. })
        | (Trigger::BecomesUntapped(filter), GameEvent::ObjectUntapped { object, .. }) => state
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
                known,
                ..
            },
        ) => {
            eval::players(*who, state, you).is_some_and(|players| players.contains(player))
                && (matches!(filter, baylee_cards_dsl::Filter::Any)
                    || (*known
                        && state
                            .source_object(baylee_core::ids::DamageSourceRef {
                                object: *object,
                                version: *version,
                            })
                            .is_some_and(|card| {
                                eval::matches_with_context(filter, state, card, you, context)
                            })))
        }
        (Trigger::BecomesTarget, _) => targeting(event, state, source).is_some(),
        (Trigger::EnchantedBecomesTarget, _) => state
            .object(source)
            .and_then(|o| o.attached_to)
            .and_then(|host| targeting(event, state, host))
            .is_some(),
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
        (
            Trigger::BlocksOrBecomesBlockedByOneOrMore(filter),
            GameEvent::BecameBlocker {
                object: blocker,
                attacker,
            },
        ) => {
            let hit = |b: ObjectId, a: ObjectId| -> bool {
                let other = if b == source {
                    a
                } else if a == source {
                    b
                } else {
                    return false;
                };
                state
                    .object(other)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
            };
            let Some(at) = event_groups::index_of_event(event, batch) else {
                return false;
            };
            hit(*blocker, *attacker)
                && !batch[..at].iter().any(|entry| {
                    matches!(
                        entry.event,
                        GameEvent::BecameBlocker {
                            object: b,
                            attacker: a,
                        } if (b == source) == (*blocker == source) && hit(b, a)
                    )
                })
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
            let Some(at) = event_groups::index_of_event(event, batch) else {
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
                step: began_step,
                active,
                ..
            },
        ) => {
            begins_step(state, *began_step, *step)
                && state
                    .object(source)
                    .and_then(crate::object::GameObject::chosen_opponent)
                    == Some(*active)
        }
        (
            Trigger::StepBegin { step, whose },
            GameEvent::StepChanged {
                step: began_step,
                active,
                ..
            },
        ) => {
            if !begins_step(state, *began_step, *step) {
                return false;
            }
            match whose {
                PlayerRel::You => *active == you,
                // "Each opponent's upkeep" is the upkeep of whichever
                // opponent is active, which is what "an opponent's" says.
                PlayerRel::Opponent | PlayerRel::EachOpponent => state.is_opponent(*active, you),
                // "At the beginning of the upkeep of enchanted land's
                // controller" (Cursed Land): that controller's step only.
                PlayerRel::ControllerOfAttached => {
                    crate::eval::controller_of_attached(state, source) == Some(*active)
                }
                PlayerRel::EachPlayer | PlayerRel::ActivePlayer => true,
                // Every other relation names no step's player. This was a
                // `_ => true`, and Miser's Cage's "each opponent's upkeep"
                // fired in its controller's own upkeep too;
                // `every_step_trigger_names_a_player_the_matcher_reads`
                // keeps the pool off this arm.
                _ => false,
            }
        }
        (Trigger::SearchesLibrary(rel), GameEvent::LibrarySearched { player }) => {
            eval::players(*rel, state, you).is_some_and(|seats| seats.contains(player))
        }
        (
            Trigger::GainsLife(rel) | Trigger::LosesLife(rel),
            GameEvent::LifeChanged {
                player, old, new, ..
            },
        ) => {
            (if matches!(trigger, Trigger::LosesLife(_)) {
                new < old
            } else {
                new > old
            }) && eval::players(*rel, state, you).is_some_and(|seats| seats.contains(player))
        }
        (
            Trigger::DealsCombatDamage(filter),
            GameEvent::DamageDealt {
                source: Some(damage_source),
                amount: 1..,
                is_combat: true,
                ..
            },
        ) => {
            first_combat_damage_by(event, batch, *damage_source)
                && state
                    .object(*damage_source)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        (
            Trigger::DealsCombatDamageToPlayerOrPlaneswalker(filter),
            GameEvent::DamageDealt {
                source: Some(damage_source),
                target,
                amount: 1..,
                is_combat: true,
            },
        ) => {
            let valid_target = match target {
                crate::event::DamageTarget::Player(_) => true,
                crate::event::DamageTarget::Object(id) => {
                    state.object_or_departed(*id).is_some_and(|o| {
                        o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::PLANESWALKER)
                    })
                }
            };
            valid_target
                && state
                    .object(*damage_source)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, you, context))
        }
        _ => false,
    }
}
