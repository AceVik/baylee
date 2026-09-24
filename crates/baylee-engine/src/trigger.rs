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
                    if matches(trigger, &entry.event, state, emblem, obj.controller) {
                        let times = trigger_count(state, trigger, emblem, obj.controller)
                            * repeats(&entry.event);
                        let event_object = event_object_of(&entry.event);
                        for _ in 0..times {
                            triggers.push(PendingTrigger {
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
                    triggers.push(PendingTrigger {
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
                    } = &entry.event
                        && *object == permanent
                        && *from == crate::zone::Zone::Battlefield
                        && *to == crate::zone::Zone::Graveyard
                    {
                        triggers.push(PendingTrigger {
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
                if matches(trigger, &entry.event, state, permanent, obj.controller) {
                    let event_object = event_object_of(&entry.event);
                    for _ in 0..repeats(&entry.event) {
                        triggers.push(PendingTrigger {
                            source: permanent,
                            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                            controller: obj.controller,
                            timestamp: obj.timestamp,
                            event_object,
                            implicit_target: None,
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
                let (target_obj, caster) = match &entry.event {
                    GameEvent::SpellCast { object, player } => (*object, Some(*player)),
                    GameEvent::AbilityTriggered {
                        object, controller, ..
                    } => (*object, Some(*controller)),
                    _ => continue,
                };
                let targets_this = state
                    .object(target_obj)
                    .is_some_and(|o| o.targets_object(permanent));
                if targets_this && caster != Some(obj.controller) {
                    triggers.push(PendingTrigger {
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
        for (index, ability) in abilities.iter().enumerate() {
            let Some(firing) = triggered_parts(ability) else {
                continue;
            };
            let trigger = firing.trigger;
            if !all_kinds && !matches!(trigger, Trigger::LeavesBattlefield(_) | Trigger::Dies(_)) {
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
                if matches(trigger, &entry.event, state, permanent, obj.controller) {
                    let times = trigger_count(state, trigger, permanent, obj.controller)
                        * repeats(&entry.event);
                    let event_object = event_object_of(&entry.event);
                    for _ in 0..times {
                        triggers.push(PendingTrigger {
                            source: permanent,
                            ability_index: index as u32,
                            abilities: Some(list),
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
                    // That is a trigger of its own and not the default:
                    // Storm the Vault is an unimplemented stub, no card in
                    // the pool encodes it, and the transcoder refuses the
                    // reference's batch modes outright, so nothing was
                    // relying on the accident. Giving it a `Trigger` variant
                    // is what the card will want, not this line.
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
/// The number is multiplied with [`trigger_count`], not confused with it:
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
            .object_or_departed(*object)
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
            .object_or_departed(*object)
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
            .object_or_departed(*object)
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
                .is_some_and(|o| o.targets_object(source))
                || matches!(event, GameEvent::AbilityTriggered { object, .. } if {
                    state.object(*object).is_some_and(|o| o.targets_object(source))
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
            PlayerRel::Opponent => state.is_opponent(*player, you),
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
                PlayerRel::Opponent => state.is_opponent(*player, you),
                _ => true,
            };
            // "except the first one they draw in each of their draw steps"
            // is about a draw *step*, not about a turn. A draw on somebody
            // else's turn is never in this player's draw step, so it always
            // fires — which is the case Orcish Bowmasters is played for
            // (Brainstorm, Rhystic Study, a Howling Mine on my turn), and
            // the old `count > 1` read it as the opponent's excepted first
            // draw and fired nothing at all.
            //
            // The exception left: a draw during their own upkeep makes the
            // draw-step draw the second of the turn, and that one fires
            // although it is the step's first. Closing it wants a per-step
            // counter beside `per_turn.draws`.
            let their_draw_step =
                state.turn.active == *player && state.turn.step == crate::turn::Step::Draw;
            player_matches && !(their_draw_step && count <= 1)
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
                _ => true,
            }
        }
        _ => false,
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
                count: 3
            }),
            3
        );
        assert_eq!(
            repeats(&GameEvent::CardsDrawn {
                player: me(),
                count: 0
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
}
