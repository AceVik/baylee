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
use baylee_cards_dsl::{AbilityDef, Condition, PlayerRel, StepKind, Trigger};
use baylee_core::ids::{ObjectId, PlayerId};

/// A triggered ability waiting to go on the stack.
#[derive(Clone, Debug)]
pub struct PendingTrigger {
    /// The permanent whose ability triggered.
    pub source: ObjectId,
    /// Index into the source card's abilities.
    pub ability_index: u32,
    /// Event-time rules text. Captured even while the source is on the
    /// battlefield: a legend choice or lethal SBA can remove a copy before
    /// this trigger is stacked. Synthetic keyword triggers carry their own
    /// effects instead and leave this `None`.
    pub abilities: Option<crate::object::AbilityList>,
    /// Controller of the trigger.
    pub controller: PlayerId,
    /// Timestamp of the source (stable same-controller ordering).
    pub timestamp: u64,
    /// The object the triggering event was about (if any).
    pub event_object: Option<ObjectId>,
    /// The event permanent's mana value before leaving the battlefield.
    pub event_mana_value: Option<u32>,
    /// The player a damage event dealt damage to, and how much: "that
    /// player" and "that much" of a combat-damage trigger (Questing Beast).
    pub event_damage: Option<(PlayerId, u16)>,
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
    /// Whether it fires at most once each turn.
    once_per_turn: bool,
    /// The intervening-`if` clause, if the card prints one (CR 603.4).
    condition: Option<Condition>,
}

const fn triggered_parts(ability: &'static AbilityDef) -> Option<Firing> {
    match ability {
        AbilityDef::Triggered {
            trigger,
            once_per_turn,
            condition,
            ..
        }
        | AbilityDef::ModalTriggered {
            trigger,
            once_per_turn,
            condition,
            ..
        } => Some(Firing {
            trigger,
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
            let Some(abilities) = obj.own_abilities else {
                continue;
            };
            for (index, ability) in abilities.iter().enumerate() {
                let Some(firing) = triggered_parts(ability) else {
                    continue;
                };
                let trigger = firing.trigger;
                // CR 603.4's first check, as in the battlefield loop below.
                if !eval::intervening_if(state, firing.condition, obj.controller, emblem) {
                    continue;
                }
                for entry in events {
                    let hit = hits(trigger, &entry.event, events, state, emblem, obj.controller);
                    if hit > 0 {
                        let times = trigger_count(state, trigger, emblem, obj.controller) * hit;
                        let event_object = event_object_for(trigger, &entry.event, emblem);
                        let event_damage = event_damage_of(&entry.event);
                        for _ in 0..times {
                            triggers.push(PendingTrigger {
                                event_mana_value: None,
                                event_damage,
                                source: emblem,
                                ability_index: index as u32,
                                abilities: Some(crate::object::AbilityList {
                                    abilities,
                                    printed: obj.own_face,
                                }),
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
    monarch_triggers(state, events, &mut triggers);
    cast_this_spell_triggers(state, lookup, events, &mut triggers);
    watch_triggers(state, events, &mut triggers);
    replicate_triggers(state, events, &mut triggers);
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
    // And the objects that are in no zone at all any more, because they
    // ceased to exist on the way (CR 111.7, CR 704.5d). Their abilities still
    // fire: the token is gone, the trigger is not. Without this pass a token
    // with a dies trigger of its own was the one permanent that could not see
    // its own death — the loop above walks zone *lists*, and a swept token
    // has left those too.
    let departed: Vec<ObjectId> = state.ceased.iter().map(|o| o.id).collect();
    collect_for_objects(state, lookup, &departed, events, false, &mut triggers);
    for trigger in &mut triggers {
        trigger.event_mana_value = trigger.event_object.and_then(|id| {
            state
                .ltb_mana_values
                .iter()
                .find(|(object, _)| *object == id)
                .map(|(_, value)| *value)
        });
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

/// Replicate's copies, one entry for each payment the trigger can copy for:
/// the trigger lists the first `n` of them.
///
/// A slice of one table and not a count read at resolution, because the
/// count is the cast's and is fixed as it is cast (CR 702.56a): read off the
/// spell later, it would be whatever a later cast of the same card wrote.
/// As long as the cast wizard's bound ([`X_CEILING`]), so no count the
/// question can offer is cut short here.
///
/// [`X_CEILING`]: crate::engine::cast_wizard::X_CEILING
static REPLICATE_COPIES: [baylee_cards_dsl::Effect;
    crate::engine::cast_wizard::X_CEILING as usize] =
    [baylee_cards_dsl::Effect::CopyThisSpell; crate::engine::cast_wizard::X_CEILING as usize];

/// Replicate's triggered ability (CR 702.56a): "when you cast this spell, if
/// a replicate cost was paid for it, copy it for each time its replicate cost
/// was paid".
///
/// A triggered ability of the *spell*, which functions on the stack, so no
/// walk over permanents finds it: it is read off `SpellCast` directly, as
/// the monarch's are read off the events. A spell whose cost was paid no
/// times does not trigger at all — the intervening "if" (CR 603.4) — and a
/// copy is never cast (CR 707.10), so never triggers it again. The spell is
/// the ability's source and its implicit first target, which is what
/// [`baylee_cards_dsl::Effect::CopyThisSpell`] copies; its controller is the
/// player who cast it.
fn replicate_triggers(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for entry in events {
        let GameEvent::SpellCast { object, player } = entry.event else {
            continue;
        };
        let Some(spell) = state.object(object).filter(|o| o.replicated > 0) else {
            continue;
        };
        let copies = usize::from(spell.replicated).min(REPLICATE_COPIES.len());
        triggers.push(PendingTrigger {
            event_damage: None,
            event_mana_value: None,
            source: object,
            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
            abilities: None,
            controller: player,
            timestamp: spell.timestamp,
            event_object: Some(object),
            implicit_target: Some(object),
            synthetic_effects: Some(&REPLICATE_COPIES[..copies]),
            once_per_turn: false,
            synthetic_target: None,
            chosen_mode: None,
        });
    }
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
fn watch_triggers(
    state: &GameState,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for watch in &state.delayed {
        let crate::state::DelayedWhen::DiesOrIsExiled { card, after, .. } = watch.when else {
            continue;
        };
        let crate::state::DelayedAction::Trigger { source, effects } = watch.action else {
            continue;
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
        if !matches!(left, Some(Zone::Graveyard | Zone::Exile)) {
            continue;
        }
        triggers.push(PendingTrigger {
            event_damage: None,
            event_mana_value: None,
            source,
            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
            abilities: None,
            controller: watch.controller,
            timestamp: state.object(source).map_or(0, |o| o.timestamp),
            event_object: Some(card),
            implicit_target: None,
            synthetic_effects: Some(effects),
            once_per_turn: false,
            synthetic_target: None,
            chosen_mode: None,
        });
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
    let Some(at) = batch
        .iter()
        .position(|entry| std::ptr::eq(&raw const entry.event, event))
    else {
        return false;
    };
    batch[..at]
        .iter()
        .rev()
        .find_map(|entry| match entry.event {
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

/// Whether a combat `DamageDealt` is the first of its combat damage step to
/// reach `object`. All combat damage in a step is dealt at once (CR 510.2),
/// so however many creatures dealt it, `object` was dealt damage in one
/// event, and "whenever this creature is dealt damage" triggers once
/// (CR 603.2c). One step's damage is one batch: nothing is scanned between
/// its events, and first strike's damage is another step.
fn first_combat_damage_to(
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
    object: ObjectId,
) -> bool {
    let Some(at) = batch
        .iter()
        .position(|entry| std::ptr::eq(&raw const entry.event, event))
    else {
        return false;
    };
    !batch[..at].iter().any(|entry| {
        matches!(
            entry.event,
            GameEvent::DamageDealt {
                target: crate::event::DamageTarget::Object(dealt),
                amount,
                is_combat: true,
                ..
            } if dealt == object && amount > 0
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
        event_mana_value: None,
        event_damage: None,
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

/// Undying (CR 702.93a) and persist (CR 702.79a) as the one sentence each
/// of them is: "return it to the battlefield under its owner's control with
/// a +1/+1 (or -1/-1) counter on it".
///
/// `TargetSpec::EventObject` and not a target: neither keyword prints the
/// word, so nothing may be chosen and nothing can be made illegal by a
/// hexproof granted in response — the ability returns the card that just
/// died or it returns nothing.
static UNDYING_RETURN: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::return_to_owner_with(
        baylee_cards_dsl::TargetSpec::EventObject,
        baylee_cards_dsl::CounterKind::P1P1,
        1,
    )];

static PERSIST_RETURN: &[baylee_cards_dsl::Effect] =
    &[baylee_cards_dsl::Effect::return_to_owner_with(
        baylee_cards_dsl::TargetSpec::EventObject,
        baylee_cards_dsl::CounterKind::M1M1,
        1,
    )];

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

/// The largest generic ward cost [`WARD_PAY_OR_COUNTER`] can charge.
///
/// Read by `keyword_tests::every_ward_in_the_pool_is_one_the_engine_charges`,
/// which is what turns "the table is long enough" from a thing somebody
/// remembers into a build failure the day a set prints a bigger one.
#[cfg(test)]
pub(crate) const WARD_CEILING: usize = WARD_PAY_OR_COUNTER.len() - 1;

/// The player a damage event dealt damage to and the amount, if it is one
/// dealt to a player.
fn event_damage_of(event: &GameEvent) -> Option<(PlayerId, u16)> {
    match event {
        GameEvent::DamageDealt {
            target: crate::event::DamageTarget::Player(player),
            amount,
            ..
        } => Some((*player, *amount)),
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
fn event_object_for(trigger: &Trigger, event: &GameEvent, source: ObjectId) -> Option<ObjectId> {
    match (trigger, event) {
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

/// The object an event is about, if any.
fn event_object_of(event: &GameEvent) -> Option<ObjectId> {
    match event {
        GameEvent::ZoneChanged { object, .. }
        | GameEvent::SpellCast { object, .. }
        | GameEvent::AbilityTriggered { object, .. }
        | GameEvent::BecameTarget { object, .. }
        | GameEvent::PlayerBecameTarget { object, .. }
        | GameEvent::BecameAttacker { object, .. }
        | GameEvent::BecameBlocker { object, .. }
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
        }
        | GameEvent::AbilityTriggered {
            object, controller, ..
        } => state
            .object(object)
            .filter(|o| o.targets_object(target))
            .map(|_| (object, controller)),
        _ => None,
    }
}

/// "When you cast this spell" (cascade, CR 702.85a): a trigger condition
/// that cannot trigger from the battlefield functions where it can, which is
/// the stack (CR 113.6k). Each spell cast in this batch is asked for its own
/// `Trigger::SpellCast(&Filter::This)` abilities, and only for those — its
/// other abilities do not function there.
fn cast_this_spell_triggers(
    state: &GameState,
    lookup: &impl CardLookup,
    events: &[crate::event::JournalEntry],
    triggers: &mut Vec<PendingTrigger>,
) {
    for entry in events {
        let GameEvent::SpellCast { object, player } = entry.event else {
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
            if !matches!(
                firing.trigger,
                baylee_cards_dsl::Trigger::SpellCast(baylee_cards_dsl::Filter::This)
            ) {
                continue;
            }
            if !eval::intervening_if(state, firing.condition, player, object) {
                continue;
            }
            triggers.push(PendingTrigger {
                event_mana_value: None,
                source: object,
                ability_index: index as u32,
                abilities: Some(list),
                controller: player,
                timestamp: spell.timestamp,
                event_object: Some(object),
                implicit_target: None,
                synthetic_effects: None,
                once_per_turn: firing.once_per_turn,
                synthetic_target: None,
                chosen_mode: None,
                event_damage: None,
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
fn hits(
    trigger: &Trigger,
    event: &GameEvent,
    batch: &[crate::event::JournalEntry],
    state: &GameState,
    source: ObjectId,
    you: PlayerId,
) -> u32 {
    match trigger {
        Trigger::TargetedByOpponent {
            filter,
            you: counts_you,
        } => targeted_by_opponent(event, state, filter, *counts_you, source, you) * repeats(event),
        Trigger::DrawsExceptFirst(rel) => match event {
            GameEvent::CardsDrawn {
                player,
                first_in_draw_step,
                ..
            } if match rel {
                PlayerRel::You => *player == you,
                PlayerRel::Opponent => state.is_opponent(*player, you),
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
        _ => u32::from(matches(trigger, event, batch, state, source, you)) * repeats(event),
    }
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
    source: ObjectId,
) -> bool {
    let Some(o) = state.object_or_departed(object) else {
        return false;
    };
    let Some(was) = state.last_known_characteristics(object) else {
        return eval::matches(filter, state, o, you, source);
    };
    let mut as_it_was = o.clone();
    as_it_was.counters = state
        .ltb_counters
        .iter()
        .find(|(id, _)| *id == object)
        .map(|(_, counters)| counters.clone())
        .unwrap_or_default();
    eval::matches_projected(filter, state, &as_it_was, was, you, source)
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
    source: ObjectId,
    you: PlayerId,
) -> u32 {
    let fits = |target: ObjectId| {
        state.object(target).is_some_and(|o| {
            o.zone == Zone::Battlefield && eval::matches(filter, state, o, you, source)
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
            let objects = targets.into_iter().filter(|t| fits(*t)).count();
            let player =
                counts_you && (obj.target_players.contains(you) || obj.chosen_player == Some(you));
            u32::try_from(objects).unwrap_or(u32::MAX) + u32::from(player)
        }
        _ => 0,
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
        // `None` on everything that was never a copy: the printed list did
        // not change, so asking the object is asking the same question.
        let looked_back = if all_kinds {
            None
        } else {
            state
                .ltb_abilities
                .iter()
                .find(|(id, _)| *id == permanent)
                .map(|(_, list)| *list)
        };
        // Not `obj.card`: a token has none, and bailing out here is how every
        // token on the battlefield used to be invisible to triggers —
        // including its own.
        let list = looked_back.unwrap_or_else(|| obj.ability_list(lookup));
        let abilities = list.abilities;
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
                    for _ in 0..times_triggered(
                        state,
                        baylee_cards_dsl::TriggerEventKind::Any,
                        permanent,
                    ) {
                        triggers.push(PendingTrigger {
                            event_mana_value: None,
                            event_damage: None,
                            source: permanent,
                            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                            controller: obj.controller,
                            timestamp: obj.timestamp,
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
        // Undying (CR 702.93a) and persist (CR 702.79a). Engine-level
        // keyword triggers like prowess above, and for the same reason: the
        // bit is on the face, the rule is one sentence, and writing that
        // sentence onto each card would make the keyword decorative — a
        // card claiming `KeywordSet::UNDYING` would look supported and do
        // nothing (`keyword_tests::no_card_claims_a_keyword_the_engine_ignores`).
        //
        // Only on the look-back pass, because the permanent is in a
        // graveyard by the time anything asks. `all_kinds` is the
        // battlefield scan and there is nothing there for this to match.
        if !all_kinds {
            // The **printed** bits and not the projected ones, and that is a
            // limit rather than a choice: `move_object` clears `obj.cache`
            // (CR 400.7), so `characteristics()` has already fallen back to
            // `base` by the time this scan runs, and a continuous effect that
            // *granted* undying stopped applying when the permanent left the
            // battlefield. Closing it needs a fourth look-back store holding
            // the projected keywords, the way `ltb_counters` holds the
            // counters. Mikaeus, the Unhallowed is the one card in this pool
            // that wants it and is `Coverage::Partial` for three reasons of
            // which this is one.
            let keywords = obj.characteristics().keywords;
            for (bit, kind, effects) in [
                (
                    baylee_cards_dsl::KeywordSet::UNDYING,
                    baylee_cards_dsl::CounterKind::P1P1,
                    UNDYING_RETURN,
                ),
                (
                    baylee_cards_dsl::KeywordSet::PERSIST,
                    baylee_cards_dsl::CounterKind::M1M1,
                    PERSIST_RETURN,
                ),
            ] {
                if !keywords.contains(bit) {
                    continue;
                }
                // A token has no card to return (CR 111.7): it ceases to
                // exist as a state-based action and the object in `ceased`
                // is all that is left of it. The trigger would resolve onto
                // nothing, which is a rule that looks broken rather than
                // one that declines.
                if obj.card.is_none() {
                    continue;
                }
                // The intervening `if` (CR 603.4), and it is read out of
                // the look-back store rather than off the object: the
                // counters were cleared by the very move this trigger is
                // about. Checked **once** and not twice, which is the one
                // place this engine departs from the letter of 603.4 — the
                // rule says on trigger and again on resolution, and both
                // reads are of the same frozen last-known information, so
                // the second cannot answer differently.
                let had = state
                    .ltb_counters
                    .iter()
                    .find(|(id, _)| *id == permanent)
                    .map_or(0, |(_, counters)| counters.get(kind));
                if had > 0 {
                    continue;
                }
                for entry in events {
                    if let GameEvent::ZoneChanged {
                        object,
                        from,
                        to,
                        cause: _,
                        place: _,
                    } = &entry.event
                        && *object == permanent
                        && *from == crate::zone::Zone::Battlefield
                        && *to == crate::zone::Zone::Graveyard
                    {
                        for _ in 0..times_triggered(
                            state,
                            baylee_cards_dsl::TriggerEventKind::Any,
                            permanent,
                        ) {
                            triggers.push(PendingTrigger {
                                event_mana_value: None,
                                event_damage: None,
                                source: permanent,
                                ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                                controller: obj.controller,
                                timestamp: obj.timestamp,
                                event_object: Some(permanent),
                                implicit_target: Some(permanent),
                                abilities: None,
                                synthetic_effects: Some(effects),
                                once_per_turn: false,
                                synthetic_target: None,
                                chosen_mode: None,
                            });
                        }
                    }
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
            if !crate::effects::applies_to(state, fx, obj) {
                continue;
            }
            for entry in events {
                let hit = hits(
                    trigger,
                    &entry.event,
                    events,
                    state,
                    permanent,
                    obj.controller,
                );
                if hit > 0 {
                    let event_object = event_object_for(trigger, &entry.event, permanent);
                    let times = trigger_count(state, trigger, permanent, obj.controller) * hit;
                    for _ in 0..times {
                        triggers.push(PendingTrigger {
                            event_mana_value: None,
                            event_damage: None,
                            source: permanent,
                            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                            controller: obj.controller,
                            timestamp: obj.timestamp,
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
        for ability in abilities {
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
                        triggers.push(PendingTrigger {
                            event_mana_value: None,
                            event_damage: None,
                            source: permanent,
                            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                            controller: obj.controller,
                            timestamp: obj.timestamp,
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
            let trigger = firing.trigger;
            // Off the battlefield, the triggers that look back (CR 603.10a)
            // and "when you cycle this card", which triggers from wherever
            // the card winds up (CR 702.29c).
            if !all_kinds
                && !matches!(
                    trigger,
                    Trigger::LeavesBattlefield(_) | Trigger::Dies(_) | Trigger::CycledThis
                )
            {
                continue;
            }
            // CR 603.4, the first of its two checks: an ability whose
            // intervening-`if` clause is false does not trigger at all.
            // Before `trigger_count`, so a trigger multiplier has nothing
            // to double — Panharmonicon doubles a trigger, not a
            // non-trigger.
            if !eval::intervening_if(state, firing.condition, obj.controller, permanent) {
                continue;
            }
            for entry in events {
                let hit = hits(
                    trigger,
                    &entry.event,
                    events,
                    state,
                    permanent,
                    obj.controller,
                );
                if hit > 0 {
                    let times = trigger_count(state, trigger, permanent, obj.controller) * hit;
                    let event_object = event_object_for(trigger, &entry.event, permanent);
                    let event_damage = event_damage_of(&entry.event);
                    for _ in 0..times {
                        triggers.push(PendingTrigger {
                            event_mana_value: None,
                            event_damage,
                            source: permanent,
                            ability_index: index as u32,
                            abilities: Some(list),
                            controller: obj.controller,
                            timestamp: obj.timestamp,
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
        Trigger::EntersBattlefield(_) => baylee_cards_dsl::TriggerEventKind::EntersBattlefield,
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
fn times_triggered(
    state: &GameState,
    event_kind: baylee_cards_dsl::TriggerEventKind,
    source: ObjectId,
) -> u32 {
    let Some(source_obj) = state.object(source) else {
        return 1;
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
    source: ObjectId,
    you: PlayerId,
) -> bool {
    match (trigger, event) {
        // CR 701.27e for the second: the ability is read off the face the
        // permanent shows right after it turned over, which is the face that
        // prints it. CR 702.29c for the third: the card cycled is the
        // source, wherever it wound up.
        (Trigger::TurnedFaceUp, GameEvent::TurnedFaceUp { object })
        | (Trigger::TransformsIntoThis, GameEvent::Transformed { object, .. })
        | (Trigger::CycledThis, GameEvent::Cycled { object, .. }) => *object == source,
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
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
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
        ) => departed_matches(filter, state, *object, you, source),
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
                    .is_some_and(|o| eval::matches(filter, state, o, you, source))
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
                    .is_some_and(|o| eval::matches(filter, state, o, you, source))
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
                && (!*is_combat || first_combat_damage_to(event, batch, *dealt))
                && state
                    .object(*dealt)
                    .is_some_and(|o| eval::matches(filter, state, o, you, source))
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
            },
        ) => *object == source && changed == kind && *old < u16::from(*n) && u16::from(*n) <= *new,
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
                && first_mana_of_a_tap(event, batch, *tapped)
                && state
                    .object(*tapped)
                    .is_some_and(|o| eval::matches(filter, state, o, you, source))
        }
        // Any permanent the filter matches: City of Brass's `Filter::This`
        // is its source, Lifetap's is a Forest an opponent controls, and
        // Psychic Venom's the land it enchants.
        (Trigger::BecomesTapped(filter), GameEvent::ObjectTapped { object, .. }) => state
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
        (Trigger::Draws(rel), GameEvent::CardsDrawn { player, .. })
        | (Trigger::PlaysLand(rel), GameEvent::LandPlayed { player, .. }) => match rel {
            PlayerRel::You => *player == you,
            PlayerRel::Opponent => state.is_opponent(*player, you),
            _ => true,
        },
        (Trigger::Attacks(filter), GameEvent::BecameAttacker { object, .. }) => state
            .object(*object)
            .is_some_and(|o| eval::matches(filter, state, o, you, source)),
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
                .is_some_and(|o| eval::matches(filter, state, o, you, source))
        }
        (Trigger::AttacksAlone(filter), GameEvent::BecameAttacker { object, .. }) => {
            batch
                .iter()
                .filter(|entry| matches!(entry.event, GameEvent::BecameAttacker { .. }))
                .count()
                == 1
                && state
                    .object(*object)
                    .is_some_and(|o| eval::matches(filter, state, o, you, source))
        }
        (Trigger::FirstNoncreatureSpellCast(rel), GameEvent::SpellCast { object, player }) => {
            let player_matches = match rel {
                PlayerRel::You => *player == you,
                PlayerRel::Opponent => state.is_opponent(*player, you),
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
            // "That player": the one the event dealt damage to. With no such
            // event it stays unbound and offers nothing.
            baylee_cards_dsl::TargetSpec::ObjectOfEventPlayer(filter) => match self.event_damage {
                Some((player, _)) => {
                    baylee_cards_dsl::TargetSpec::ObjectControlledBy(filter, player)
                }
                None => spec,
            },
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectKind;
    use crate::state::ReplacementEntry;
    use baylee_cards_dsl::{Filter, ReplacementRule, TriggerEventKind};
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn me() -> PlayerId {
        PlayerId::new(0)
    }
    fn them() -> PlayerId {
        PlayerId::new(1)
    }

    fn state() -> GameState {
        let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index;
        let entry = DeckEntry {
            card: forest,
            print: baylee_core::ids::PrintRef::new(0),
        };
        let seat = || SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: (0..60).map(|_| entry).collect(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            format: FormatId::Freeform,
            seed: 12,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![seat(), seat()],
        };
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    fn permanent(state: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        )
    }

    fn rule(state: &mut GameState, controller: PlayerId, rule: ReplacementRule) {
        let name = state.names.intern("Panharmonicon");
        let source = state.create_bare(
            controller,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        state.replacement_rules.push(ReplacementEntry {
            source,
            controller,
            rule,
        });
    }

    /// "At the beginning of the upkeep of enchanted land's controller"
    /// (Cursed Land) is that controller's upkeep, whoever controls the Aura
    /// (CR 303.4e) — and nobody's once the Aura enchants nothing. Before
    /// `PlayerRel::ControllerOfAttached` was sorted into `whose`, anything
    /// that was not `You` or `Opponent` fired on every player's step.
    #[test]
    fn an_upkeep_of_the_enchanted_permanents_controller_is_theirs_alone() {
        let mut state = state();
        let land = permanent(&mut state, them(), "Forest");
        let aura = permanent(&mut state, me(), "Cursed Land");
        state.object_mut(aura).expect("just made").attached_to = Some(land);
        let upkeep = GameEvent::StepChanged {
            phase: crate::turn::Phase::Beginning,
            step: crate::turn::Step::Upkeep,
        };
        let cursed = Trigger::StepBegin {
            step: baylee_cards_dsl::StepKind::Upkeep,
            whose: PlayerRel::ControllerOfAttached,
        };

        state.turn.active = them();
        assert_eq!(
            hits(&cursed, &upkeep, &[], &state, aura, me()),
            1,
            "the land's controller's upkeep, though the Aura is mine"
        );
        state.turn.active = me();
        assert_eq!(
            hits(&cursed, &upkeep, &[], &state, aura, me()),
            0,
            "not the Aura controller's own"
        );
        state.turn.active = them();
        state.object_mut(aura).expect("still here").attached_to = None;
        assert_eq!(
            hits(&cursed, &upkeep, &[], &state, aura, me()),
            0,
            "an Aura attached to nothing has no enchanted land's controller"
        );
    }

    /// "Whenever a Forest an opponent controls becomes tapped" (Lifetap) is
    /// about another permanent than the trigger's source, which the tapped
    /// trigger answered only for its source; and the tapped permanent is the
    /// event's object, "that land" of Psychic Venom.
    #[test]
    fn a_tapped_trigger_answers_for_any_permanent_its_filter_names() {
        let mut state = state();
        let lifetap = permanent(&mut state, me(), "Lifetap");
        let (theirs, mine) = (
            permanent(&mut state, them(), "Forest"),
            permanent(&mut state, me(), "Forest"),
        );
        let tapped = |object| GameEvent::ObjectTapped {
            object,
            cause: crate::event::Cause::Cost,
        };
        let theirs_tapped = Trigger::BecomesTapped(&Filter::ControlledByOpponent);
        assert_eq!(
            hits(&theirs_tapped, &tapped(theirs), &[], &state, lifetap, me()),
            1
        );
        assert_eq!(
            hits(&theirs_tapped, &tapped(mine), &[], &state, lifetap, me()),
            0
        );
        let own = Trigger::BecomesTapped(&Filter::This);
        assert_eq!(
            hits(&own, &tapped(theirs), &[], &state, lifetap, me()),
            0,
            "City of Brass"
        );
        assert_eq!(hits(&own, &tapped(lifetap), &[], &state, lifetap, me()), 1);
        assert_eq!(event_object_of(&tapped(theirs)), Some(theirs), "that land");
    }

    /// Hypnotic Specter's "deals damage to an opponent" is any damage, where
    /// the combat trigger sees combat damage only. Fungusaur's "is dealt
    /// damage" is one event for all the combat damage of a step (CR 510.2,
    /// 603.2c), one for each other damage event, and none for damage
    /// prevented to nothing (CR 603.2g).
    #[test]
    fn damage_to_an_opponent_and_damage_dealt_to_this_count_their_events() {
        use crate::event::{DamageTarget, JournalEntry};
        let mut state = state();
        let specter = permanent(&mut state, me(), "Hypnotic Specter");
        let fungusaur = permanent(&mut state, me(), "Fungusaur");
        let (one, two) = (
            permanent(&mut state, them(), "Blocker"),
            permanent(&mut state, them(), "Blocker"),
        );
        let dealt = |source, target, amount, is_combat| GameEvent::DamageDealt {
            source: Some(source),
            target,
            amount,
            is_combat,
        };

        let specter_trigger = Trigger::DealsDamageToOpponent(&Filter::This);
        let to_them = dealt(specter, DamageTarget::Player(them()), 1, false);
        assert_eq!(
            hits(&specter_trigger, &to_them, &[], &state, specter, me()),
            1,
            "damage out of combat"
        );
        assert_eq!(
            hits(
                &Trigger::DealsCombatDamageToOpponent(&Filter::This),
                &to_them,
                &[],
                &state,
                specter,
                me()
            ),
            0,
            "which the combat trigger does not see"
        );
        let to_me = dealt(specter, DamageTarget::Player(me()), 1, false);
        assert_eq!(
            hits(&specter_trigger, &to_me, &[], &state, specter, me()),
            0
        );

        let fungus = Trigger::DealtDamage(&Filter::This);
        let entry = |seq, event| JournalEntry { seq, event };
        let blocked = vec![
            entry(1, dealt(one, DamageTarget::Object(fungusaur), 1, true)),
            entry(2, dealt(two, DamageTarget::Object(fungusaur), 1, true)),
        ];
        let fired = |batch: &[JournalEntry], at: usize| {
            hits(&fungus, &batch[at].event, batch, &state, fungusaur, me())
        };
        assert_eq!(fired(&blocked, 0), 1, "blocked by two, dealt damage once");
        assert_eq!(
            fired(&blocked, 1),
            0,
            "the second blocker's is the same event"
        );
        let burned = vec![
            entry(1, dealt(one, DamageTarget::Object(fungusaur), 1, false)),
            entry(2, dealt(two, DamageTarget::Object(fungusaur), 1, false)),
        ];
        assert_eq!(
            fired(&burned, 0) + fired(&burned, 1),
            2,
            "two spells, two events"
        );
        let prevented = vec![entry(
            1,
            dealt(one, DamageTarget::Object(fungusaur), 0, false),
        )];
        assert_eq!(fired(&prevented, 0), 0, "prevented damage was not dealt");
        let elsewhere = vec![entry(1, dealt(one, DamageTarget::Object(two), 1, false))];
        assert_eq!(fired(&elsewhere, 0), 0, "another creature's damage");
    }

    /// "Except the first one they draw in each of their draw steps" skips
    /// one card of an entry and no more: each card is its own draw
    /// (CR 121.2), so "draw three" that opens an opponent's draw step fires
    /// twice. The exception is this trigger's alone, and "an opponent" still
    /// leaves its controller's own draws out.
    #[test]
    fn the_draw_step_exception_skips_one_card_of_an_entry_and_no_more() {
        let mut state = state();
        let source = permanent(&mut state, me(), "Bowmasters");
        let drew = |player, count, first_in_draw_step| GameEvent::CardsDrawn {
            player,
            count,
            first_in_draw_step,
        };
        let fired =
            |trigger: &Trigger, event: &GameEvent| hits(trigger, event, &[], &state, source, me());
        let bowmasters = Trigger::DrawsExceptFirst(PlayerRel::Opponent);
        assert_eq!(fired(&bowmasters, &drew(them(), 1, true)), 0, "the one");
        assert_eq!(
            fired(&bowmasters, &drew(them(), 3, true)),
            2,
            "the other two"
        );
        assert_eq!(fired(&bowmasters, &drew(them(), 3, false)), 3);
        assert_eq!(fired(&bowmasters, &drew(me(), 3, false)), 0, "my own draws");
        assert_eq!(
            fired(&Trigger::Draws(PlayerRel::Opponent), &drew(them(), 3, true)),
            3,
            "a plain draw trigger has no exception"
        );
    }

    /// How many *happenings* an event is, which is not how many times an
    /// ability triggers for one of them. "Draw two cards" is one journal
    /// entry carrying a count, and an ability watching a card being drawn
    /// has to see both — a `break` that fires once for a list of matching
    /// events is a defect this shape hides from, because here the batch is
    /// a field.
    #[test]
    fn a_batched_event_is_as_many_happenings_as_it_counts() {
        assert_eq!(
            repeats(&GameEvent::CardsDrawn {
                player: me(),
                count: 3,
                first_in_draw_step: false,
            }),
            3
        );
        assert_eq!(
            repeats(&GameEvent::CardsDrawn {
                player: me(),
                count: 0,
                first_in_draw_step: false,
            }),
            1,
            "a zero would suppress a trigger that matched rather than \
             over-fire it, which is the silent direction"
        );
        assert_eq!(
            repeats(&GameEvent::Shuffled {
                player: me(),
                zone: Zone::Library
            }),
            1,
            "every other event is one happening"
        );
    }

    /// Panharmonicon: the ability triggers an *additional* time, so two of
    /// them make three triggers and not four. The rule is read against the
    /// trigger's source with the replacement's own controller as "you".
    #[test]
    fn a_trigger_multiplier_adds_a_trigger_rather_than_doubling_them() {
        let mut state = state();
        let bear = permanent(&mut state, me(), "Bear");

        assert_eq!(trigger_count(&state, &Trigger::ETB, bear, me()), 1);

        for _ in 0..2 {
            rule(
                &mut state,
                me(),
                ReplacementRule::TriggerMultiplier {
                    source_filter: &Filter::ControlledByYou,
                    event: TriggerEventKind::EntersBattlefield,
                },
            );
        }
        assert_eq!(
            trigger_count(&state, &Trigger::ETB, bear, me()),
            3,
            "two Panharmonicons are two additional triggers"
        );

        let theirs = permanent(&mut state, them(), "Their Bear");
        assert_eq!(
            trigger_count(&state, &Trigger::ETB, theirs, me()),
            1,
            "it multiplies the permanents its own controller has"
        );
    }

    /// The event kind is part of the rule: a Panharmonicon named for
    /// entering the battlefield says nothing about a dies trigger, while
    /// `Any` is the shape that covers everything.
    #[test]
    fn a_multiplier_named_for_one_event_kind_leaves_the_others_alone() {
        let mut state = state();
        let bear = permanent(&mut state, me(), "Bear");
        rule(
            &mut state,
            me(),
            ReplacementRule::TriggerMultiplier {
                source_filter: &Filter::ControlledByYou,
                event: TriggerEventKind::EntersBattlefield,
            },
        );

        assert_eq!(trigger_count(&state, &Trigger::ETB, bear, me()), 2);
        assert_eq!(
            trigger_count(&state, &Trigger::Dies(&Filter::This), bear, me()),
            1,
            "a dies trigger is not an enters-the-battlefield one"
        );

        rule(
            &mut state,
            me(),
            ReplacementRule::TriggerMultiplier {
                source_filter: &Filter::ControlledByYou,
                event: TriggerEventKind::Any,
            },
        );
        assert_eq!(
            trigger_count(&state, &Trigger::Dies(&Filter::This), bear, me()),
            2,
            "and `Any` reaches the one the named rule did not"
        );
        assert_eq!(trigger_count(&state, &Trigger::ETB, bear, me()), 3);
    }

    /// Suppression is not a multiplier of nought — it wins outright, from
    /// either side of the list, because a trigger that does not happen
    /// cannot be multiplied afterwards.
    #[test]
    fn suppression_beats_any_number_of_multipliers() {
        for suppress_first in [true, false] {
            let mut state = state();
            let bear = permanent(&mut state, me(), "Bear");
            let multiplier = ReplacementRule::TriggerMultiplier {
                source_filter: &Filter::ControlledByYou,
                event: TriggerEventKind::Any,
            };
            let suppress = ReplacementRule::TriggerSuppress {
                source_filter: &Filter::ControlledByYou,
                event: TriggerEventKind::Any,
            };
            if suppress_first {
                rule(&mut state, me(), suppress);
                rule(&mut state, me(), multiplier);
            } else {
                rule(&mut state, me(), multiplier);
                rule(&mut state, me(), suppress);
            }
            assert_eq!(
                trigger_count(&state, &Trigger::ETB, bear, me()),
                0,
                "suppression first: {suppress_first}"
            );
        }
    }

    /// A source that is no longer there answers one. It is the look-back
    /// case — a dies trigger is collected after the permanent has left —
    /// and nought there would be a trigger silently dropped.
    #[test]
    fn a_source_that_is_gone_triggers_once() {
        let mut state = state();
        let bear = permanent(&mut state, me(), "Bear");
        rule(
            &mut state,
            me(),
            ReplacementRule::TriggerMultiplier {
                source_filter: &Filter::ControlledByYou,
                event: TriggerEventKind::Any,
            },
        );
        assert_eq!(trigger_count(&state, &Trigger::ETB, bear, me()), 2);

        state.arena.remove(bear);
        assert_eq!(trigger_count(&state, &Trigger::ETB, bear, me()), 1);
    }

    /// Prowess is a triggered ability (CR 702.108a), so a multiplier adds a
    /// trigger to it as it does to a printed one (CR 603.2d), and a
    /// suppressor takes it away. The keyword's trigger is built here rather
    /// than read off the card, and it used to be pushed once whatever the
    /// board said: Katara, the Fearless doubled Sokka's token and left his
    /// prowess at +1/+1.
    #[test]
    fn a_keyword_trigger_is_multiplied_like_a_printed_one() {
        fn prowess_triggers(state: &GameState, from: u64) -> usize {
            collect(state, &RegistryLookup, from)
                .iter()
                .filter(|t| t.synthetic_effects == Some(PROWESS_PUMP))
                .count()
        }
        let mut state = state();
        let monk = permanent(&mut state, me(), "Monk");
        std::sync::Arc::make_mut(&mut state.object_mut(monk).expect("the monk").base).keywords =
            baylee_cards_dsl::KeywordSet::PROWESS;
        let name = state.names.intern("Opt");
        let spell = state.create_bare(me(), ObjectKind::Spell, name, ZoneLocation::Stack);
        let from = state.journal.len() as u64;
        state.journal.record(GameEvent::SpellCast {
            object: spell,
            player: me(),
        });
        assert_eq!(prowess_triggers(&state, from), 1, "one spell, one prowess");

        rule(
            &mut state,
            me(),
            ReplacementRule::TriggerMultiplier {
                source_filter: &Filter::ControlledByYou,
                event: TriggerEventKind::Any,
            },
        );
        assert_eq!(
            prowess_triggers(&state, from),
            2,
            "a multiplier adds a prowess trigger"
        );

        rule(
            &mut state,
            me(),
            ReplacementRule::TriggerSuppress {
                source_filter: &Filter::ControlledByYou,
                event: TriggerEventKind::Any,
            },
        );
        assert_eq!(
            prowess_triggers(&state, from),
            0,
            "a suppressor stops prowess as well"
        );
    }
}
