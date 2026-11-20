//! Amplify (CR 702.38a, CR 702.38b, CR 614.1c, CR 614.12, CR 614.12a, CR 614.16):
//! pre-entry hand reveal of cards sharing a creature type, +1/+1 counter
//! placement before state-based actions, separate prompting for multiple
//! instances, exclusion of co-entering hand cards, Changeling interaction, and
//! counter-doubling replacement interaction.

use super::*;
use crate::choice::ChoicePrompt;
use crate::event::{Cause, GameEvent};
use crate::zone::ZonePosition;
use baylee_cards_dsl::{CounterKind, EnterModifier, Filter, Modifier, static_ability};
use baylee_core::generated::subtypes;

const AMPLIFY_1_ZOMBIE: u32 = 7460;
const AMPLIFY_3_DRAGON: u32 = 7461;
const ZERO_ZERO_ZOMBIE: u32 = 7462;
const DOUBLE_AMPLIFY_ZOMBIE: u32 = 7463;
const PLAIN_ZOMBIE: u32 = 7464;
const PLAIN_DRAGON: u32 = 7465;
const PLAIN_BEAST: u32 = 7466;
const CHANGELING_INSTANT: u32 = 7467;
const DOUBLER: u32 = 7468;

static ZOMBIE_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[subtypes::creature::ZOMBIE];
static DRAGON_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[subtypes::creature::DRAGON];
static BEAST_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[subtypes::creature::BEAST];

static AMPLIFY_1: &[EnterModifier] = &[EnterModifier::Amplify(1)];
static AMPLIFY_3: &[EnterModifier] = &[EnterModifier::Amplify(3)];
static DOUBLE_AMPLIFY_1: &[EnterModifier] = &[EnterModifier::Amplify(1), EnterModifier::Amplify(1)];
static DOUBLER_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::Any,
    Modifier::DoubleCountersPlacedByYou,
)];

fn amplify_cards() -> Vec<&'static CardDef> {
    vec![
        card(
            AMPLIFY_1_ZOMBIE,
            FaceDef {
                subtypes: ZOMBIE_SUBTYPES,
                enter_modifiers: AMPLIFY_1,
                ..creature_face("Amplify 1 Zombie", "{0}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            AMPLIFY_3_DRAGON,
            FaceDef {
                subtypes: DRAGON_SUBTYPES,
                enter_modifiers: AMPLIFY_3,
                ..creature_face("Amplify 3 Dragon", "{0}", 5, 5)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            ZERO_ZERO_ZOMBIE,
            FaceDef {
                subtypes: ZOMBIE_SUBTYPES,
                enter_modifiers: AMPLIFY_1,
                ..creature_face("Zero Zero Zombie", "{0}", 0, 0)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            DOUBLE_AMPLIFY_ZOMBIE,
            FaceDef {
                subtypes: ZOMBIE_SUBTYPES,
                enter_modifiers: DOUBLE_AMPLIFY_1,
                ..creature_face("Double Amplify Zombie", "{0}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            PLAIN_ZOMBIE,
            FaceDef {
                subtypes: ZOMBIE_SUBTYPES,
                ..creature_face("Plain Zombie", "{0}", 2, 2)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            PLAIN_DRAGON,
            FaceDef {
                subtypes: DRAGON_SUBTYPES,
                ..creature_face("Plain Dragon", "{0}", 5, 5)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            PLAIN_BEAST,
            FaceDef {
                subtypes: BEAST_SUBTYPES,
                ..creature_face("Plain Beast", "{0}", 3, 3)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            CHANGELING_INSTANT,
            face("Changeling Instant", "{0}", TypeSet::INSTANT),
            KeywordSet::CHANGELING,
            &[],
        ),
        card(
            DOUBLER,
            face("Counter Doubler", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            DOUBLER_ABILITIES,
        ),
    ]
}

fn reveal_events(engine: &Bench, who: PlayerId) -> Vec<Vec<ObjectId>> {
    engine
        .journal()
        .entries()
        .iter()
        .filter_map(|entry| match &entry.event {
            GameEvent::Revealed { player, cards } if *player == who => Some(cards.clone()),
            _ => None,
        })
        .collect()
}

fn cast(engine: &mut Bench, seat: PlayerId, card: ObjectId) {
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("offered spell casts cleanly");
}

fn walk_to_amplify(engine: &mut Bench) -> (PlayerId, Vec<ObjectId>, u8, u8) {
    walk_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Amplify,
                ..
            }
        )
    });
    match engine.pending().clone() {
        Pending::ChooseCards {
            player,
            options,
            min,
            max,
            prompt: ChoicePrompt::Amplify,
            ..
        } => (player, options, min, max),
        other => panic!("expected ChoicePrompt::Amplify, got {other:?}"),
    }
}

#[test]
fn amplify_1_reveals_zero_or_one_matching_card_and_records_reveal_event() {
    // Reveal 0 for Amplify 1: enters with 0 counters and records no reveal event.
    let mut engine = bench(
        7460,
        amplify_cards(),
        [
            Seat::default().holding(&[AMPLIFY_1_ZOMBIE, PLAIN_ZOMBIE, PLAIN_ZOMBIE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let amp = the(&engine, ZoneLocation::Hand(me()), AMPLIFY_1_ZOMBIE);
    let zombies = objects(&engine, ZoneLocation::Hand(me()), PLAIN_ZOMBIE);
    cast(&mut engine, me(), amp);
    let (player, options, min, max) = walk_to_amplify(&mut engine);
    assert_eq!((player, min, max), (me(), 0, 2));
    assert_eq!(options.len(), 2);
    assert!(zombies.iter().all(|id| options.contains(id)));
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(amp)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    assert_eq!(
        (
            engine.state().object(amp).unwrap().characteristics().power,
            engine
                .state()
                .object(amp)
                .unwrap()
                .characteristics()
                .toughness
        ),
        (Some(2), Some(2))
    );
    assert!(reveal_events(&engine, me()).is_empty());

    // Reveal 1 of 2 Zombies for Amplify 1: 1 counter, reveal event logged, both remain in hand.
    let mut engine = bench(
        7461,
        amplify_cards(),
        [
            Seat::default().holding(&[AMPLIFY_1_ZOMBIE, PLAIN_ZOMBIE, PLAIN_ZOMBIE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let amp = the(&engine, ZoneLocation::Hand(me()), AMPLIFY_1_ZOMBIE);
    let zombies = objects(&engine, ZoneLocation::Hand(me()), PLAIN_ZOMBIE);
    cast(&mut engine, me(), amp);
    let (_, options, _, _) = walk_to_amplify(&mut engine);
    assert!(options.contains(&zombies[0]));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![zombies[0]],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(amp)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(reveal_events(&engine, me()), vec![vec![zombies[0]]]);
    assert_eq!(
        objects(&engine, ZoneLocation::Hand(me()), PLAIN_ZOMBIE).len(),
        2,
        "revealed cards remain in hand (CR 701.20b)"
    );
}

#[test]
fn amplify_3_reveals_multiple_matching_cards_and_records_reveal_event() {
    // Reveal 2 Dragons for Amplify 3: 6 +1/+1 counters (11/11).
    let mut engine = bench(
        7462,
        amplify_cards(),
        [
            Seat::default().holding(&[AMPLIFY_3_DRAGON, PLAIN_DRAGON, PLAIN_DRAGON]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let dragon = the(&engine, ZoneLocation::Hand(me()), AMPLIFY_3_DRAGON);
    let dragons = objects(&engine, ZoneLocation::Hand(me()), PLAIN_DRAGON);
    cast(&mut engine, me(), dragon);
    let (_, options, min, max) = walk_to_amplify(&mut engine);
    assert_eq!((min, max), (0, 2));
    assert_eq!(options.len(), 2);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: dragons.clone(),
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(dragon)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        6
    );
    assert_eq!(
        (
            engine
                .state()
                .object(dragon)
                .unwrap()
                .characteristics()
                .power,
            engine
                .state()
                .object(dragon)
                .unwrap()
                .characteristics()
                .toughness
        ),
        (Some(11), Some(11))
    );
    assert_eq!(reveal_events(&engine, me()), vec![dragons]);
}

#[test]
fn amplify_excludes_non_matching_types_and_includes_changeling_cards() {
    let mut engine = bench(
        7463,
        amplify_cards(),
        [
            Seat::default().holding(&[
                AMPLIFY_1_ZOMBIE,
                PLAIN_ZOMBIE,
                CHANGELING_INSTANT,
                PLAIN_BEAST,
                PLAIN_DRAGON,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let amp = the(&engine, ZoneLocation::Hand(me()), AMPLIFY_1_ZOMBIE);
    let zombie = the(&engine, ZoneLocation::Hand(me()), PLAIN_ZOMBIE);
    let changeling = the(&engine, ZoneLocation::Hand(me()), CHANGELING_INSTANT);
    let beast = the(&engine, ZoneLocation::Hand(me()), PLAIN_BEAST);
    let dragon = the(&engine, ZoneLocation::Hand(me()), PLAIN_DRAGON);

    cast(&mut engine, me(), amp);
    let (_, options, min, max) = walk_to_amplify(&mut engine);
    assert_eq!((min, max), (0, 2));
    assert!(options.contains(&zombie));
    assert!(
        options.contains(&changeling),
        "Changeling noncreature card shares every creature type (CR 702.38a, CR 702.73a)"
    );
    assert!(!options.contains(&beast));
    assert!(!options.contains(&dragon));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![zombie, changeling],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(amp)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );

    // No matching cards in hand -> settles cleanly with zero prompts and zero counters.
    let mut engine = bench(
        7464,
        amplify_cards(),
        [
            Seat::default().holding(&[AMPLIFY_1_ZOMBIE, PLAIN_BEAST]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let amp = the(&engine, ZoneLocation::Hand(me()), AMPLIFY_1_ZOMBIE);
    cast(&mut engine, me(), amp);
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(amp)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
}

#[test]
fn amplify_zero_toughness_creature_survives_with_reveal_and_dies_without() {
    // Revealing 1 Zombie puts the +1/+1 counter on BEFORE SBAs -> survives as 1/1.
    let mut engine = bench(
        7465,
        amplify_cards(),
        [
            Seat::default().holding(&[ZERO_ZERO_ZOMBIE, PLAIN_ZOMBIE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let remains = the(&engine, ZoneLocation::Hand(me()), ZERO_ZERO_ZOMBIE);
    let zombie = the(&engine, ZoneLocation::Hand(me()), PLAIN_ZOMBIE);
    cast(&mut engine, me(), remains);
    let (_, options, _, _) = walk_to_amplify(&mut engine);
    assert_eq!(options, vec![zombie]);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![zombie],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    let obj = engine.state().object(remains).unwrap();
    assert_eq!(
        obj.zone,
        Zone::Battlefield,
        "0/0 Amplify creature survives when counters are placed as it enters (CR 614.12)"
    );
    assert_eq!(obj.counters.get(CounterKind::P1P1), 1);
    assert_eq!(
        (obj.characteristics().power, obj.characteristics().toughness),
        (Some(1), Some(1))
    );

    // Declining to reveal -> enters 0/0 -> dies immediately to SBA 704.5f.
    let mut engine = bench(
        7466,
        amplify_cards(),
        [
            Seat::default().holding(&[ZERO_ZERO_ZOMBIE, PLAIN_ZOMBIE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let remains = the(&engine, ZoneLocation::Hand(me()), ZERO_ZERO_ZOMBIE);
    cast(&mut engine, me(), remains);
    let _ = walk_to_amplify(&mut engine);
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(remains).unwrap().zone,
        Zone::Graveyard,
        "0/0 Amplify creature with 0 reveals dies to SBA 704.5f"
    );
}

#[test]
fn multiple_amplify_instances_prompt_separately_and_can_reveal_same_card() {
    let mut engine = bench(
        7467,
        amplify_cards(),
        [
            Seat::default().holding(&[DOUBLE_AMPLIFY_ZOMBIE, PLAIN_ZOMBIE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let double = the(&engine, ZoneLocation::Hand(me()), DOUBLE_AMPLIFY_ZOMBIE);
    let zombie = the(&engine, ZoneLocation::Hand(me()), PLAIN_ZOMBIE);

    cast(&mut engine, me(), double);

    // First Amplify 1 prompt.
    let (_, first_options, _, _) = walk_to_amplify(&mut engine);
    assert_eq!(first_options, vec![zombie]);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![zombie],
            },
        )
        .unwrap();

    // Second Amplify 1 prompt (CR 702.38b: each instance works separately).
    let (_, second_options, _, _) = walk_to_amplify(&mut engine);
    assert_eq!(
        second_options,
        vec![zombie],
        "same card in hand can be revealed again for the second Amplify instance"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![zombie],
            },
        )
        .unwrap();

    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(double)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        2
    );
    assert_eq!(
        reveal_events(&engine, me()),
        vec![vec![zombie], vec![zombie]]
    );
}

#[test]
fn simultaneous_entry_excludes_co_entering_hand_card() {
    let mut engine = bench(
        7468,
        amplify_cards(),
        [
            Seat::default().holding(&[AMPLIFY_1_ZOMBIE, AMPLIFY_1_ZOMBIE, PLAIN_ZOMBIE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let entering = objects(&engine, ZoneLocation::Hand(me()), AMPLIFY_1_ZOMBIE);
    let (z1, z2) = (entering[0], entering[1]);
    let staying = the(&engine, ZoneLocation::Hand(me()), PLAIN_ZOMBIE);

    // Move z1 and z2 from hand onto the battlefield simultaneously in one entry event.
    let seq = engine.state().journal.last_seq();
    engine.state.entries.begin_event(seq, Some(me()));
    let _ = engine.state.move_object(
        z1,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    );
    let _ = engine.state.move_object(
        z2,
        ZoneLocation::Battlefield,
        ZonePosition::Top,
        Cause::Effect,
    );

    let mut prompts = 0usize;
    while let Some(pending) = crate::entry::advance(&mut engine.state) {
        match pending {
            Pending::ChooseCards {
                options,
                prompt: ChoicePrompt::EntryOrder,
                ..
            } => {
                crate::entry::answer(
                    &mut engine.state,
                    &PlayerAction::ChooseObjects {
                        objects: vec![options[0]],
                    },
                );
            }
            Pending::ChooseCards {
                options,
                prompt: ChoicePrompt::Amplify,
                ..
            } => {
                prompts += 1;
                assert_eq!(
                    options,
                    vec![staying],
                    "neither co-entering card nor self may be revealed (CR 702.38a)"
                );
                crate::entry::answer(
                    &mut engine.state,
                    &PlayerAction::ChooseObjects {
                        objects: vec![staying],
                    },
                );
            }
            other => panic!("unexpected entry pending: {other:?}"),
        }
    }
    assert_eq!(prompts, 2, "both co-entering Amplify creatures prompted");
    assert_eq!(
        engine
            .state()
            .object(z1)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(
        engine
            .state()
            .object(z2)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
}

#[test]
fn doubling_season_doubles_amplify_counters() {
    let mut engine = bench(
        7469,
        amplify_cards(),
        [
            Seat::with(&[DOUBLER]).holding(&[AMPLIFY_3_DRAGON, PLAIN_DRAGON, PLAIN_DRAGON]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let dragon = the(&engine, ZoneLocation::Hand(me()), AMPLIFY_3_DRAGON);
    let dragons = objects(&engine, ZoneLocation::Hand(me()), PLAIN_DRAGON);

    cast(&mut engine, me(), dragon);
    let (_, options, _, _) = walk_to_amplify(&mut engine);
    assert_eq!(options.len(), 2);
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: dragons })
        .unwrap();
    settle(&mut engine, me());

    assert_eq!(
        engine
            .state()
            .object(dragon)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        12,
        "Amplify 3 x 2 revealed Dragons = 6 counters, doubled to 12 by CR 614.16"
    );
    assert_eq!(
        (
            engine
                .state()
                .object(dragon)
                .unwrap()
                .characteristics()
                .power,
            engine
                .state()
                .object(dragon)
                .unwrap()
                .characteristics()
                .toughness
        ),
        (Some(17), Some(17))
    );
}
