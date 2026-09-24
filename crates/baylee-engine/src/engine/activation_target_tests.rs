//! How many targets an activated ability names (CR 601.2c, by CR 602.2b).
//!
//! An activated ability's target used to be a bare spec, and a bare spec is
//! exactly one. Four lands in the pool print something else. Wintermoon Mesa
//! taps "two target lands", and Skemfar Elderhall, Bretagard Stronghold and
//! Abstergo Entertainment each print "up to". Each property below is a
//! different way to get that count wrong.
//!
//! - The offer reads the minimum. "Two target lands" is withheld while the
//!   board holds one land, and "up to one" is offered on an empty board.
//! - The question reads both bounds, and one answer is refused as too few.
//! - "Up to one" answered with nothing is an answer. The activation moves on
//!   to its cost and does not ask the same question again (CR 115.6).
//! - "Up to one" answered with nothing still has a target requirement. A
//!   counter it puts on "target creature" goes nowhere, and not onto the
//!   source the way an untargeted "put a counter on it" would.
//! - An ability activated from a hand is offered on the same target probe as
//!   one on the battlefield. Rustic Clachan's reinforce once was not.

use super::testkit::*;
use super::*;
use crate::event::Cause;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 115;

fn forest() -> CardIndex {
    basic_forest()
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}
fn aurochs() -> CardIndex {
    card_index("3961ef7c-4eb4-482e-9cda-d49d6a29c5a9")
}
fn wintermoon_mesa() -> CardIndex {
    card_index("a4a6f95e-856c-4eb5-82ba-b2406be22b23")
}
fn skemfar_elderhall() -> CardIndex {
    card_index("70965b80-c8ad-4718-ae20-12a4d8228898")
}
fn rustic_clachan() -> CardIndex {
    card_index("cde68428-0033-4ede-92f1-ab91de0a41fb")
}
fn yawgmoth() -> CardIndex {
    card_index("a1e232c0-dc38-47be-a5a0-f68bc1d86a29")
}
fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// The sacrifice ability's place on both lands: after the mana ability.
const SACRIFICE: u32 = 1;

fn is_tapped(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .expect("the permanent is still on the table")
        .status
        .contains(crate::object::Status::TAPPED)
}

fn offered(engine: &Engine<RegistryLookup>) -> Vec<(ObjectId, u32)> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.abilities.clone()
}

/// Starts a game on `mine` against `theirs` and stops in seat 0's first main
/// phase with every mana source floated but `keep`, the land whose own
/// ability is about to be pressed.
fn board(
    mine: &[CardIndex],
    theirs: &[CardIndex],
    hand: &[CardIndex],
    keep: Option<CardIndex>,
) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .hand(0, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let kept = keep.and_then(|card| on_battlefield(&engine, p0, card));
    tap_mana_where(&mut engine, p0, |id| Some(id) != kept);
    engine
}

fn activate(engine: &mut Engine<RegistryLookup>, source: ObjectId) {
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ActivateAbility {
                source,
                ability_index: SACRIFICE,
            },
        )
        .expect("the offer listed it");
}

fn tokens(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_none())
        })
        .collect()
}

/// CR 601.2c: "tap two target lands" needs two lands to name, and the offer
/// says so before anything is pressed.
///
/// The two boards differ by one land on the other side of the table. Both
/// pay the same {2} from the same Sol Ring, so the price cannot be what
/// withholds the ability on the first board. The Mesa itself is a land and
/// is on the battlefield while its targets are chosen, so it counts: one
/// Forest across the table makes two.
#[test]
fn two_target_lands_are_withheld_until_the_board_holds_two() {
    let p0 = PlayerId::new(0);
    let alone = board(
        &[wintermoon_mesa(), sol_ring()],
        &[],
        &[],
        Some(wintermoon_mesa()),
    );
    let mesa = on_battlefield(&alone, p0, wintermoon_mesa()).expect("the Mesa stands");
    assert!(
        !is_tapped(&alone, mesa),
        "the Mesa is untapped, so {{T}} can be paid"
    );
    assert_eq!(
        alone.state().players[0].mana_pool.total(),
        2,
        "{{2}} is floating"
    );
    assert!(
        !offered(&alone).contains(&(mesa, SACRIFICE)),
        "one land on the battlefield is not two targets"
    );

    let paired = board(
        &[wintermoon_mesa(), sol_ring()],
        &[forest()],
        &[],
        Some(wintermoon_mesa()),
    );
    let mesa = on_battlefield(&paired, p0, wintermoon_mesa()).expect("the Mesa stands");
    assert_eq!(paired.state().players[0].mana_pool.total(), 2);
    assert!(
        offered(&paired).contains(&(mesa, SACRIFICE)),
        "the Mesa and one Forest are two lands"
    );
}

/// The question asks for exactly two, refuses one, and the answer taps both.
#[test]
fn two_target_lands_are_asked_for_as_two_and_both_are_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board(
        &[wintermoon_mesa(), sol_ring()],
        &[forest(), swamp()],
        &[],
        Some(wintermoon_mesa()),
    );
    let mesa = on_battlefield(&engine, p0, wintermoon_mesa()).expect("the Mesa stands");
    let forest = on_battlefield(&engine, p1, forest()).expect("their Forest");
    let swamp = on_battlefield(&engine, p1, swamp()).expect("their Swamp");

    activate(&mut engine, mesa);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (2, 2), "two target lands is exactly two");
    assert!(options.contains(&forest) && options.contains(&swamp) && options.contains(&mesa));

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![forest],
                    players: vec![],
                },
            )
            .is_err(),
        "one land is too few"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![forest, swamp],
                players: vec![],
            },
        )
        .expect("two lands is the answer");
    assert_eq!(
        engine.state().object(mesa).map(|o| o.zone),
        Some(Zone::Graveyard),
        "the Mesa was sacrificed as the cost"
    );
    assert!(
        !is_tapped(&engine, forest) && !is_tapped(&engine, swamp),
        "nothing happens before it resolves"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, forest), "the first target is tapped");
    assert!(is_tapped(&engine, swamp), "and so is the second");
}

/// CR 115.6: "up to one" with nothing to name is offered, and it asks
/// nothing. A question with only the empty answer is not put to anyone.
#[test]
fn up_to_one_with_nothing_to_name_is_offered_and_asks_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = board(
        &[
            skemfar_elderhall(),
            swamp(),
            swamp(),
            forest(),
            forest(),
            forest(),
        ],
        &[],
        &[],
        Some(skemfar_elderhall()),
    );
    let hall = on_battlefield(&engine, p0, skemfar_elderhall()).expect("the Elderhall stands");
    assert!(!is_tapped(&engine, hall));
    assert!(
        offered(&engine).contains(&(hall, SACRIFICE)),
        "no creature to shrink is no reason to withhold the Elves"
    );

    activate(&mut engine, hall);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "nothing is asked, and the ability is on the stack: {:?}",
        engine.pending()
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(tokens(&engine, p0).len(), 2, "two Elf Warriors");
}

/// "Up to one" answered with nothing is an answer, and it is not asked for
/// again.
///
/// `start_activation` is re-entered with the chosen list after every
/// question. An empty list cannot tell "answered with nothing" from "not
/// asked yet", so a check on the list asked the same question forever. The
/// Aurochs is the control: it was offered, it was not named, and it keeps
/// its size.
#[test]
fn up_to_one_answered_with_nothing_is_not_asked_again() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board(
        &[
            skemfar_elderhall(),
            swamp(),
            swamp(),
            forest(),
            forest(),
            forest(),
        ],
        &[aurochs()],
        &[],
        Some(skemfar_elderhall()),
    );
    let hall = on_battlefield(&engine, p0, skemfar_elderhall()).expect("the Elderhall stands");
    let aurochs = on_battlefield(&engine, p1, aurochs()).expect("their Aurochs");
    let printed = pt(&engine, aurochs);

    activate(&mut engine, hall);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 1), "up to one is none or one");
    assert!(
        options.contains(&aurochs),
        "a creature seat 0 does not control"
    );
    assert!(!options.contains(&hall), "a land is not a creature");

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .expect("none is a legal answer");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the answer moved the activation on, and nothing is asked again: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().object(hall).map(|o| o.zone),
        Some(Zone::Graveyard),
        "the cost was paid"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, aurochs), printed, "nobody named the Aurochs");
    assert_eq!(tokens(&engine, p0).len(), 2);
}

/// Named, the one target gets -2/-2. Named and gone before resolution, the
/// ability does not resolve at all (CR 608.2b): every target it had is
/// illegal, so the Elves the second sentence makes are not made either.
#[test]
fn up_to_one_named_shrinks_it_and_named_then_gone_does_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for bounced in [false, true] {
        let mut engine = board(
            &[
                skemfar_elderhall(),
                swamp(),
                swamp(),
                forest(),
                forest(),
                forest(),
            ],
            &[aurochs()],
            &[],
            Some(skemfar_elderhall()),
        );
        let hall = on_battlefield(&engine, p0, skemfar_elderhall()).expect("the Elderhall stands");
        let aurochs = on_battlefield(&engine, p1, aurochs()).expect("their Aurochs");
        let (power, toughness) = pt(&engine, aurochs);
        assert!(
            toughness > 2,
            "a body that survives -2/-2, so the size can be read"
        );
        activate(&mut engine, hall);
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![aurochs],
                    players: vec![],
                },
            )
            .expect("the Aurochs was on the menu");
        if bounced {
            engine
                .dev_state_mut(p0)
                .expect("a test seat has dev commands")
                .move_object(
                    aurochs,
                    ZoneLocation::Hand(p1),
                    ZonePosition::Top,
                    Cause::DevCommand,
                )
                .expect("the Aurochs moves");
        }
        pass_until(&mut engine, stack_is_empty);
        if bounced {
            assert!(
                tokens(&engine, p0).is_empty(),
                "its only target is gone, so the ability does not resolve"
            );
        } else {
            assert_eq!(
                pt(&engine, aurochs),
                (power - 2, toughness - 2),
                "-2/-2 until end of turn"
            );
            assert_eq!(tokens(&engine, p0).len(), 2);
        }
    }
}

/// An ability activated from a hand is offered on the same target probe as
/// one on the battlefield (CR 602.2b applies 601.2c to both).
///
/// Rustic Clachan's reinforce was offered on a board with no creature, and
/// the press was then refused with "no legal targets". That is the two
/// probes disagreeing, which the client draws as a button that does not
/// work. The creature that makes the difference is on the other side of the
/// table, so it cannot be a mana source.
#[test]
fn a_hand_ability_is_offered_only_with_a_legal_target() {
    let p0 = PlayerId::new(0);
    for creature in [false, true] {
        let theirs: &[CardIndex] = if creature { &[aurochs()] } else { &[] };
        let engine = board(&[plains(), plains()], theirs, &[rustic_clachan()], None);
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            2,
            "{{1}}{{W}} is floating"
        );
        let card = in_hand(&engine, p0, rustic_clachan()).expect("the Clachan is in hand");
        let reinforce = offered(&engine).iter().any(|(source, _)| *source == card);
        assert_eq!(
            reinforce, creature,
            "reinforce is offered exactly when a creature is there to be reinforced"
        );
    }
}

/// "Up to one" answered with nothing resolves with no target, and its
/// target-reading effect reaches nobody (CR 115.6).
///
/// Two things are held here that an empty answer could break. The first is
/// the re-entry through the cost question: Yawgmoth asks for its target,
/// then for the creature it eats, and the second question comes back into
/// `start_activation` with an empty list in hand. The second is where the
/// counter goes. `AddCounter` falls back to the source when an ability has no
/// target requirement, which is how "put a counter on this creature" is
/// written. An ability that has one and was answered with nothing is not that
/// ability, so Yawgmoth must not put the -1/-1 counter on himself. The card is
/// still drawn, because the ability resolved.
#[test]
fn up_to_one_answered_with_nothing_counts_nobody_and_still_draws() {
    let p0 = PlayerId::new(0);
    let mut engine = board(&[yawgmoth(), llanowar_elves()], &[], &[], None);
    let physician = on_battlefield(&engine, p0, yawgmoth()).expect("Yawgmoth stands");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves stand");
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, physician);
    let Pending::ChooseTargets { min, max, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 1));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .expect("none is a legal answer");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "the cost is asked next, and the target question is not asked again: {:?}",
            engine.pending()
        )
    };
    assert_eq!(options, vec![elves], "another creature pays the cost");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elves pay");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the ability is on the stack, and nothing is asked a second time: {:?}",
        engine.pending()
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(physician)
            .map(|o| o.counters.get(baylee_cards_dsl::CounterKind::M1M1)),
        Some(0),
        "no counter fell back onto the source"
    );
    assert_eq!(pt(&engine, physician), (2, 4));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1,
        "the card is drawn: the ability resolved"
    );
}
