use super::events::event_object_of;
use super::matching::{hits, repeats};
use super::watches::first_mana_of_a_tap;
use super::*;
mod speed;
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

#[test]
fn graveyard_trigger_zone_uses_the_cards_owner_and_only_its_graveyard() {
    const UPKEEP: Trigger = Trigger::StepBegin {
        step: baylee_cards_dsl::StepKind::Upkeep,
        whose: PlayerRel::You,
    };
    static ABILITIES: &[AbilityDef] = &[
        baylee_cards_dsl::triggered!(UPKEEP, &[baylee_cards_dsl::Effect::gain_life(1)]),
        baylee_cards_dsl::triggered!(
            UPKEEP,
            &[baylee_cards_dsl::Effect::gain_life(1)],
            zone = TriggerZone::Graveyard
        ),
    ];
    for zone in [
        ZoneLocation::Battlefield,
        ZoneLocation::Graveyard(me()),
        ZoneLocation::Hand(me()),
        ZoneLocation::Library(me()),
        ZoneLocation::Exile(me()),
        ZoneLocation::Command(me()),
    ] {
        for active in [me(), them()] {
            let mut state = state();
            let card = state.zones.list(ZoneLocation::Library(me()))[0];
            state
                .move_object(
                    card,
                    zone,
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
            let object = state.object_mut(card).expect("card");
            object.take_abilities(crate::object::AbilityList::from_static(
                ABILITIES, None, None,
            ));
            object.set_controller(them());
            let version = object.version;
            rule(
                &mut state,
                them(),
                ReplacementRule::TriggerMultiplier {
                    source_filter: &Filter::ControlledByYou,
                    event: TriggerEventKind::Any,
                },
            );
            state.turn.active = active;
            let from = state.journal.len() as u64;
            state.journal.record(GameEvent::StepChanged {
                active,
                phase: crate::turn::Phase::Beginning,
                step: crate::turn::Step::Upkeep,
            });
            let triggers = collect(&state, &RegistryLookup, from);
            let expected = match zone {
                ZoneLocation::Battlefield if active == them() => {
                    Some((2, 0, them(), Some(version)))
                }
                ZoneLocation::Graveyard(_) if active == me() => Some((1, 1, me(), Some(version))),
                _ => None,
            };
            assert_eq!(
                triggers.len(),
                expected.map_or(0, |(count, ..)| count),
                "zone {zone:?}, active {active:?}"
            );
            if let Some((_, index, controller, source_version)) = expected {
                assert_eq!(
                    (
                        triggers[0].ability_index,
                        triggers[0].controller,
                        triggers[0].source_version
                    ),
                    (index, controller, source_version)
                );
            }
        }
    }
}

fn state() -> GameState {
    let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
        .expect("registry contains Forest")
        .index;
    let entry = DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
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
        rules: baylee_core::card_metadata::RulesMetadata::default(),
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
        active: them(),
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
        hits(
            &cursed,
            &GameEvent::StepChanged {
                active: me(),
                phase: crate::turn::Phase::Beginning,
                step: crate::turn::Step::Upkeep
            },
            &[],
            &state,
            aura,
            me()
        ),
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

#[test]
fn an_untapped_permanent_is_the_event_object() {
    let mut state = state();
    let artifact = permanent(&mut state, them(), "Sol Ring");
    let untapped = GameEvent::ObjectUntapped {
        object: artifact,
        cause: crate::event::Cause::Effect,
    };
    assert_eq!(
        event_object_of(&untapped),
        Some(artifact),
        "that permanent's controller"
    );
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
    let entry = |seq, event| JournalEntry {
        seq,
        event,
        departure: None,
    };
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

/// "Whenever you're dealt damage, put that many …" (Living Artifact):
/// two unblocked attackers are one event (CR 510.2, 603.2c), so the
/// ability triggers on the first of their entries only, and "that many"
/// is both creatures' damage. A source's trigger in the same batch keeps
/// its own share. Out of combat, each damage event triggers it once for
/// its own amount; damage to somebody else, and damage prevented to
/// nothing (CR 603.2g), trigger nothing.
#[test]
fn being_dealt_damage_is_one_event_per_combat_step_and_counts_all_of_it() {
    use crate::event::{DamageTarget, JournalEntry};
    let mut state = state();
    let aura = permanent(&mut state, me(), "Living Artifact");
    let (bear, ogre) = (
        permanent(&mut state, them(), "Bear"),
        permanent(&mut state, them(), "Ogre"),
    );
    let dealt = |source, player, amount, is_combat| GameEvent::DamageDealt {
        source: Some(source),
        target: DamageTarget::Player(player),
        amount,
        is_combat,
    };
    let entry = |seq, event| JournalEntry {
        seq,
        event,
        departure: None,
    };
    let you = Trigger::PlayerDealtDamage(PlayerRel::You);
    let fired =
        |batch: &[JournalEntry], at: usize| hits(&you, &batch[at].event, batch, &state, aura, me());

    let combat = vec![
        entry(1, dealt(bear, them(), 1, true)),
        entry(2, dealt(bear, me(), 2, true)),
        entry(3, dealt(ogre, me(), 3, true)),
    ];
    assert_eq!(fired(&combat, 0), 0, "somebody else's damage");
    assert_eq!(fired(&combat, 1), 1, "the step's first to me");
    assert_eq!(
        fired(&combat, 2),
        0,
        "the second attacker's is the same event"
    );
    assert_eq!(
        event_damage_of(&you, &combat[1].event, &combat),
        Some(5),
        "that many: all of the step's damage to me"
    );
    assert_eq!(
        event_damage_of(
            &Trigger::DealsCombatDamageToOpponent(&Filter::This),
            &combat[2].event,
            &combat
        ),
        Some(3),
        "a source's trigger counts its own damage"
    );

    let burned = vec![
        entry(1, dealt(bear, me(), 2, false)),
        entry(2, dealt(ogre, me(), 3, false)),
    ];
    assert_eq!(fired(&burned, 0) + fired(&burned, 1), 2, "two events");
    assert_eq!(event_damage_of(&you, &burned[1].event, &burned), Some(3));
    let prevented = vec![entry(1, dealt(bear, me(), 0, false))];
    assert_eq!(fired(&prevented, 0), 0, "prevented damage was not dealt");
    let opponents = Trigger::PlayerDealtDamage(PlayerRel::EachOpponent);
    assert_eq!(
        hits(&opponents, &combat[0].event, &combat, &state, aura, me()),
        1,
        "the relation is the trigger's"
    );
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
        drawn_this_turn_after: u32::from(count),
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
            drawn_this_turn_after: 3,
        }),
        3
    );
    assert_eq!(
        repeats(&GameEvent::CardsDrawn {
            player: me(),
            count: 0,
            first_in_draw_step: false,
            drawn_this_turn_after: 0,
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
        instant_number: 0,
        number: 1,
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
#[test]
#[allow(clippy::too_many_lines)] // paired printed and granted text replacement scenario
fn printed_and_granted_triggers_freeze_their_own_word_context() {
    use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
    use crate::object::AbilityList;
    use crate::text_changes::TextReplacement;
    use baylee_cards_dsl::{Duration, Effect, Layer, Modifier, TextWordKind};
    use baylee_core::color::{Color, ColorSet};
    use baylee_core::ids::EffectId;
    static GREEN: Filter = Filter::HasColor(ColorSet::of(Color::Green));
    static EFFECTS: &[Effect] = &[Effect::gain_life(1)];
    static PRINTED: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
        Trigger::SpellCast(&GREEN),
        EFFECTS,
    )];
    let mut state = state();
    let recipient = permanent(&mut state, me(), "Recipient");
    let grantor = permanent(&mut state, me(), "Grantor");
    state
        .object_mut(recipient)
        .unwrap()
        .take_abilities(AbilityList::from_static(PRINTED, None, None));
    let recipient_id = state.source_identity(recipient).unwrap();
    let grantor_id = state.source_identity(grantor).unwrap();
    state.text_changes.replace(
        recipient_id,
        TextReplacement {
            kind: TextWordKind::Color,
            from: 4,
            to: 3,
        },
    );
    state.text_changes.replace(
        grantor_id,
        TextReplacement {
            kind: TextWordKind::Color,
            from: 4,
            to: 1,
        },
    );
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(grantor),
        controller: me(),
        origin: EffectOrigin::Static,
        layer: Layer::Ability,
        timestamp: 1,
        duration: Duration::WhileSourceOnBattlefield,
        filter: EffectFilter::object(&state, recipient),
        modifier: Modifier::GrantTriggered {
            trigger: Trigger::SpellCast(&GREEN),
            effects: EFFECTS,
            target: None,
        },
    });
    let from = state.journal.len() as u64;
    for color in [Color::Blue, Color::Red] {
        let name = state.names.intern("Spell");
        let spell = state.create_bare(me(), ObjectKind::Spell, name, ZoneLocation::Stack);
        state.object_mut(spell).unwrap().base_mut().colors = ColorSet::of(color);
        state.journal.record(GameEvent::SpellCast {
            instant_number: 0,
            number: 1,
            object: spell,
            player: me(),
        });
    }
    let triggers = collect(&state, &RegistryLookup, from);
    assert_eq!(
        triggers.len(),
        2,
        "one printed red listener and one granted blue listener"
    );
    let printed = triggers
        .iter()
        .find(|trigger| trigger.synthetic_effects.is_none())
        .unwrap();
    let granted = triggers
        .iter()
        .find(|trigger| trigger.synthetic_effects.is_some())
        .unwrap();
    assert_eq!(printed.text.color_word(Color::Green), Color::Red);
    assert_eq!(granted.text.color_word(Color::Green), Color::Blue);
    assert_eq!(
        printed.abilities.as_ref().unwrap().base_text(0),
        printed.text
    );
    state.text_changes.replace(
        recipient_id,
        TextReplacement {
            kind: TextWordKind::Color,
            from: 3,
            to: 0,
        },
    );
    state.text_changes.replace(
        grantor_id,
        TextReplacement {
            kind: TextWordKind::Color,
            from: 1,
            to: 0,
        },
    );
    assert_eq!(printed.text.color_word(Color::Green), Color::Red);
    assert_eq!(granted.text.color_word(Color::Green), Color::Blue);
}

#[test]
fn mana_production_reads_its_activation_across_trigger_scan_boundaries() {
    let state = state();
    let object = state.zones.list(ZoneLocation::Library(me()))[0];
    let source = state.source_identity(object).unwrap();
    for tap_cost in [false, true] {
        for mana_ability in [false, true] {
            let mut journal = crate::event::Journal::default();
            journal.record(GameEvent::ObjectTapped {
                object,
                cause: crate::event::Cause::Cost,
            });
            journal.record(GameEvent::AbilityActivated {
                ability: None,
                source,
                player: me(),
                tap_cost,
                mana_ability,
            });
            journal.record(GameEvent::ManaProduced {
                player: me(),
                color: baylee_core::mana::ManaColor::Blue,
                amount: 1,
                source: Some(object),
            });
            // A cost/activation scan may already have consumed both preceding
            // rows. Production still belongs to that exact activation.
            assert_eq!(
                first_mana_of_a_tap(&journal.entries()[2].event, journal.entries(), object),
                tap_cost && mana_ability
            );
            journal.record(GameEvent::ManaProduced {
                player: me(),
                color: baylee_core::mana::ManaColor::Red,
                amount: 1,
                source: Some(object),
            });
            assert!(!first_mana_of_a_tap(
                &journal.entries()[3].event,
                journal.entries(),
                object
            ));
        }
    }
}

/// Every "at the beginning of …'s <step>" in the pool names a player the
/// step matcher reads. Any other `PlayerRel` falls to its `_ => false`
/// arm and never fires; before that arm it was `_ => true`, and Miser's
/// Cage, Pauper's Cage and Malignant Growth, written `EachOpponent`, fired
/// in their controller's own upkeep as well.
#[test]
fn every_step_trigger_names_a_player_the_matcher_reads() {
    const READ: &[&str] = &[
        "You",
        "Opponent",
        "EachOpponent",
        "ControllerOfAttached",
        "EachPlayer",
        "ActivePlayer",
    ];
    let mut seen = 0_usize;
    let mut unread = Vec::new();
    for def in baylee_cards::all() {
        let text = format!("{def:?}");
        for (at, _) in text.match_indices("StepBegin { step: ") {
            let rest = &text[at..];
            let Some(whose) = rest.find("whose: ").map(|i| &rest[i + "whose: ".len()..]) else {
                continue;
            };
            let name: String = whose
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect();
            seen += 1;
            if !READ.contains(&name.as_str()) {
                unread.push(format!("{}: {name}", def.name()));
            }
        }
    }
    assert!(
        seen >= 100,
        "only {seen} step triggers found in the pool; the walk has gone blind"
    );
    assert!(
        unread.is_empty(),
        "step triggers no matcher arm reads: {unread:?}"
    );
}
