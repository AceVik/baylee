//! Triggered abilities: collection and APNAP ordering.
//!
//! After every mutation batch the engine scans new journal entries and
//! matches them against the triggered abilities of all permanents (and,
//! later, cards in other zones). Matches are put on the stack in APNAP
//! order. Same-controller ordering currently follows timestamp; the
//! player-facing ordering choice is M2 (documented in engine-internals).

use crate::eval;
use crate::event::GameEvent;
use crate::state::{CardLookup, GameState};
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{AbilityDef, PlayerRel, StepKind, Trigger};
use baylee_core::ids::{ObjectId, PlayerId};

/// A triggered ability waiting to go on the stack.
#[derive(Clone, Debug)]
pub struct PendingTrigger {
    /// The permanent whose ability triggered.
    pub source: ObjectId,
    /// Index into the source card's abilities.
    pub ability_index: u32,
    /// Controller of the trigger.
    pub controller: PlayerId,
    /// Timestamp of the source (stable same-controller ordering).
    pub timestamp: u64,
    /// The object the triggering event was about (if any).
    pub event_object: Option<ObjectId>,
    /// Synthetic effects for engine-level keyword triggers (prowess).
    pub synthetic_effects: Option<&'static [baylee_cards_dsl::Effect]>,
    /// Target spec for synthetic triggers that need a target choice
    /// (granted triggered abilities, class levels).
    pub synthetic_target: Option<baylee_cards_dsl::TargetSpec>,
    /// Fires at most once each turn (marked by the engine after stacking).
    pub once_per_turn: bool,
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
    collect_for_objects(
        state,
        lookup,
        state.zones.list(ZoneLocation::Battlefield),
        events,
        true,
        &mut triggers,
    );
    // Emblems (command zone, CR 114.2): their triggered abilities fire
    // from the command zone.
    for seat in 0..state.players.len() {
        let p = PlayerId::new(seat as u8);
        for &emblem in state.zones.list(ZoneLocation::Command(p)) {
            let Some(obj) = state.object(emblem) else {
                continue;
            };
            let Some(abilities) = obj.emblem_abilities else {
                continue;
            };
            for (index, ability) in abilities.iter().enumerate() {
                let AbilityDef::Triggered {
                    trigger,
                    once_per_turn,
                    ..
                } = ability
                else {
                    continue;
                };
                for entry in events {
                    if matches(trigger, &entry.event, state, emblem, obj.controller) {
                        let times = trigger_count(state, trigger, emblem, obj.controller);
                        let event_object = event_object_of(&entry.event);
                        for _ in 0..times {
                            triggers.push(PendingTrigger {
                                source: emblem,
                                ability_index: index as u32,
                                controller: obj.controller,
                                timestamp: obj.timestamp,
                                event_object,
                                synthetic_effects: None,
                                once_per_turn: *once_per_turn,
                                synthetic_target: None,
                            });
                        }
                        break;
                    }
                }
            }
        }
    }
    // LTB/Dies triggers look back in time (CR 603.10): the source is no
    // longer on the battlefield when they fire.
    for seat in 0..state.players.len() {
        let p = PlayerId::new(seat as u8);
        for loc in [ZoneLocation::Graveyard(p), ZoneLocation::Exile(p)] {
            collect_for_objects(
                state,
                lookup,
                state.zones.list(loc),
                events,
                false,
                &mut triggers,
            );
        }
    }
    // APNAP: active player first, then in turn order; same controller by
    // timestamp (M2: player ordering choice).
    let active = state.turn.active;
    triggers.sort_by_key(|t| {
        let distance = (t.controller.get() + state.players.len() as u8 - active.get())
            % state.players.len() as u8;
        (distance, t.timestamp)
    });
    triggers
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
static WARD2_PAY_OR_COUNTER: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::PlayerMayPayOr {
        player: baylee_cards_dsl::PlayerRel::ControllerOfTarget,
        mana: 2,
        effect: &WARD_COUNTER,
    }];
static WARD1_PAY_OR_COUNTER: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::PlayerMayPayOr {
        player: baylee_cards_dsl::PlayerRel::ControllerOfTarget,
        mana: 1,
        effect: &WARD_COUNTER,
    }];
static WARD3_PAY_OR_COUNTER: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::PlayerMayPayOr {
        player: baylee_cards_dsl::PlayerRel::ControllerOfTarget,
        mana: 3,
        effect: &WARD_COUNTER,
    }];

/// The object an event is about, if any.
fn event_object_of(event: &GameEvent) -> Option<ObjectId> {
    match event {
        GameEvent::ZoneChanged { object, .. }
        | GameEvent::SpellCast { object, .. }
        | GameEvent::BecameAttacker { object, .. }
        | GameEvent::BecameBlocker { object, .. } => Some(*object),
        _ => None,
    }
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
    for &permanent in objects {
        let Some(obj) = state.object(permanent) else {
            continue;
        };
        let Some(card) = obj.card else { continue };
        let Some(def) = lookup.card(card.index) else {
            continue;
        };
        // Prowess (engine-level keyword trigger, CR 702.108).
        if obj
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::PROWESS)
        {
            for entry in events {
                if let GameEvent::SpellCast { object, player } = &entry.event
                    && *player == obj.controller
                    && state.object(*object).is_some_and(|spell| {
                        !spell
                            .characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                    })
                {
                    triggers.push(PendingTrigger {
                        source: permanent,
                        ability_index: u32::MAX,
                        controller: obj.controller,
                        timestamp: obj.timestamp,
                        event_object: Some(permanent),
                        synthetic_effects: Some(PROWESS_PUMP),
                        once_per_turn: false,
                        synthetic_target: None,
                    });
                    break;
                }
            }
        }
        // Granted triggered abilities (class levels): continuous effects
        // carrying GrantTriggered that apply to this permanent.
        for fx in state.effects.iter() {
            let baylee_cards_dsl::Modifier::GrantTriggered {
                trigger,
                effects,
                target,
            } = &fx.modifier
            else {
                continue;
            };
            let applies = match &fx.filter {
                crate::effects::EffectFilter::ObjectIs(id) => *id == permanent,
                crate::effects::EffectFilter::Dsl(filter) => eval::matches(
                    filter,
                    state,
                    obj,
                    fx.controller,
                    fx.source.unwrap_or(permanent),
                ),
            };
            if !applies {
                continue;
            }
            for entry in events {
                if matches(trigger, &entry.event, state, permanent, obj.controller) {
                    let event_object = event_object_of(&entry.event);
                    triggers.push(PendingTrigger {
                        source: permanent,
                        ability_index: u32::MAX,
                        controller: obj.controller,
                        timestamp: obj.timestamp,
                        event_object,
                        synthetic_effects: Some(effects),
                        synthetic_target: *target,
                        once_per_turn: false,
                    });
                    break;
                }
            }
        }
        // Ward {N} (engine-level keyword trigger, CR 702.21): an
        // opponent's spell or ability targets this permanent.
        for ability in def.abilities_for_face(obj.face_index as usize) {
            let AbilityDef::Ward { mana } = ability else {
                continue;
            };
            let Some(synthetic) = (match mana {
                1 => Some(WARD1_PAY_OR_COUNTER),
                2 => Some(WARD2_PAY_OR_COUNTER),
                3 => Some(WARD3_PAY_OR_COUNTER),
                _ => None,
            }) else {
                continue; // unsupported ward cost (colored/generic>3)
            };
            for entry in events {
                let (target_obj, caster) = match &entry.event {
                    GameEvent::SpellCast { object, player } => (*object, Some(*player)),
                    GameEvent::AbilityTriggered {
                        object, controller, ..
                    } => (*object, Some(*controller)),
                    _ => continue,
                };
                let targets_this = state
                    .object(target_obj)
                    .is_some_and(|o| o.targets.contains(&permanent));
                if targets_this && caster != Some(obj.controller) {
                    triggers.push(PendingTrigger {
                        source: permanent,
                        ability_index: u32::MAX,
                        controller: obj.controller,
                        timestamp: obj.timestamp,
                        event_object: Some(target_obj),
                        synthetic_effects: Some(synthetic),
                        once_per_turn: false,
                        synthetic_target: None,
                    });
                }
            }
        }
        for (index, ability) in def
            .abilities_for_face(obj.face_index as usize)
            .iter()
            .enumerate()
        {
            let AbilityDef::Triggered {
                trigger,
                once_per_turn,
                ..
            } = ability
            else {
                continue;
            };
            if !all_kinds && !matches!(trigger, Trigger::LeavesBattlefield(_) | Trigger::Dies(_)) {
                continue;
            }
            for entry in events {
                if matches(trigger, &entry.event, state, permanent, obj.controller) {
                    let times = trigger_count(state, trigger, permanent, obj.controller);
                    let event_object = event_object_of(&entry.event);
                    for _ in 0..times {
                        triggers.push(PendingTrigger {
                            source: permanent,
                            ability_index: index as u32,
                            controller: obj.controller,
                            timestamp: obj.timestamp,
                            event_object,
                            synthetic_effects: None,
                            once_per_turn: *once_per_turn,
                            synthetic_target: None,
                        });
                    }
                    break; // one trigger per event per ability — next event
                }
            }
        }
    }
}

/// How often a trigger fires: trigger multipliers (Panharmonicon) add,
/// suppressors (Elesh Norn) zero it out.
fn trigger_count(
    state: &GameState,
    trigger: &Trigger,
    source: ObjectId,
    controller: PlayerId,
) -> u32 {
    let Some(source_obj) = state.object(source) else {
        return 1;
    };
    let event_kind = match trigger {
        Trigger::EntersBattlefield(_) => baylee_cards_dsl::TriggerEventKind::EntersBattlefield,
        _ => baylee_cards_dsl::TriggerEventKind::Any,
    };
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
                return 0;
            }
            _ => {}
        }
    }
    let _ = controller;
    count
}

#[allow(clippy::too_many_lines)] // the trigger×event matrix is naturally one flat table
fn matches(
    trigger: &Trigger,
    event: &GameEvent,
    state: &GameState,
    source: ObjectId,
    you: PlayerId,
) -> bool {
    match (trigger, event) {
        (
            Trigger::EntersBattlefield(filter),
            GameEvent::ZoneChanged {
                object,
                to: Zone::Battlefield,
                ..
            },
        ) => state
            .object(*object)
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
        (
            Trigger::LeavesBattlefield(filter),
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                ..
            },
        ) => state
            .object(*object)
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
        (
            Trigger::ExiledFromBattlefield(filter),
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                to: Zone::Exile,
                ..
            },
        ) => state
            .object(*object)
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
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
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
        (Trigger::BecomesTapped(filter), GameEvent::ObjectTapped { object, .. }) => {
            *object == source
                && state
                    .object(*object)
                    .is_some_and(|o| eval::matches(filter, state, o, you, source))
        }
        (
            Trigger::Dies(filter),
            GameEvent::ZoneChanged {
                object,
                from: Zone::Battlefield,
                to: Zone::Graveyard,
                ..
            },
        ) => state
            .object(*object)
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
        (Trigger::SpellCast(filter), GameEvent::SpellCast { object, .. }) => state
            .object(*object)
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
        (Trigger::NthSpellCast { n, filter }, GameEvent::SpellCast { object, player }) => {
            // The per-turn counter is bumped before the event is journalled,
            // so it already includes the spell this event is about: the nth
            // spell is exactly the one that makes the count reach n. Copies
            // are put on the stack rather than cast, and so never count.
            let count = state
                .per_turn
                .spells_cast
                .get(player.get() as usize)
                .copied()
                .unwrap_or(0);
            count == u32::from(*n)
                && state
                    .object(*object)
                    .is_some_and(|o| eval::matches(filter, state, o, you, source))
        }
        (Trigger::BecomesTarget, GameEvent::SpellCast { object, .. }) => {
            state
                .object(*object)
                .is_some_and(|o| o.targets.contains(&source))
                || matches!(event, GameEvent::AbilityTriggered { object, .. } if {
                    state.object(*object).is_some_and(|o| o.targets.contains(&source))
                })
        }
        (
            Trigger::EntersBattlefieldEvoked,
            GameEvent::ZoneChanged {
                object,
                to: Zone::Battlefield,
                ..
            },
        ) => *object == source && state.object(*object).is_some_and(|o| o.alt_cast),
        (Trigger::Draws(rel), GameEvent::CardsDrawn { player, .. }) => match rel {
            PlayerRel::You => *player == you,
            PlayerRel::Opponent => *player != you,
            _ => true,
        },
        (Trigger::Attacks(filter), GameEvent::BecameAttacker { object, .. }) => state
            .object(*object)
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
        (Trigger::DrawsExceptFirst(rel), GameEvent::CardsDrawn { player, .. }) => {
            let count = state
                .per_turn
                .draws
                .get(player.get() as usize)
                .copied()
                .unwrap_or(0);
            let player_matches = match rel {
                PlayerRel::You => *player == you,
                PlayerRel::Opponent => *player != you,
                _ => true,
            };
            // The event fires from the SECOND card drawn onward.
            player_matches && count > 1
        }
        (Trigger::FirstNoncreatureSpellCast(rel), GameEvent::SpellCast { object, player }) => {
            let player_matches = match rel {
                PlayerRel::You => *player == you,
                PlayerRel::Opponent => *player != you,
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
        (Trigger::StepBegin { step, whose }, GameEvent::StepChanged { .. }) => {
            let step_matches = matches!(
                (step, event),
                (
                    StepKind::Upkeep,
                    GameEvent::StepChanged {
                        step: crate::turn::Step::Upkeep,
                        ..
                    }
                ) | (
                    StepKind::Draw,
                    GameEvent::StepChanged {
                        step: crate::turn::Step::Draw,
                        ..
                    }
                ) | (
                    StepKind::End,
                    GameEvent::StepChanged {
                        step: crate::turn::Step::End,
                        ..
                    }
                ) | (
                    StepKind::CombatBegin,
                    GameEvent::StepChanged {
                        step: crate::turn::Step::CombatBegin,
                        ..
                    }
                )
            );
            if !step_matches {
                return false;
            }
            match whose {
                PlayerRel::You => state.turn.active == you,
                PlayerRel::Opponent => state.turn.active != you,
                _ => true,
            }
        }
        _ => false,
    }
}
