//! The riders five card files used to carry as "NOT SUPPORTED yet".
//!
//! Each of them named a mechanic the card printed but the engine could not
//! do. The mechanics landed one by one; the comments did not move, so the
//! files kept advertising gaps that had been closed — the worst kind of
//! stale note, because it discourages anyone from using the card.
//!
//! These tests are what let those comments be rewritten: one per rider,
//! driving a real game to the moment the rider matters. If a rider ever
//! regresses, the header stops being a lie and starts being a test failure.

use super::testkit::{Duel, RegistryLookup, card_index, keep_mulligans, pt, reach_main_phase};
use super::*;
use crate::zone::ZoneLocation;
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_core::mana::ManaColor;

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}
fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn badlands() -> CardIndex {
    card_index("13ff3222-91cb-4796-a34e-899ed817694c")
}
fn godless_shrine() -> CardIndex {
    card_index("73864fcc-1bde-4bc0-831e-2b93e546e417")
}
fn doubling_season() -> CardIndex {
    card_index("01546b7d-a233-4176-8843-d732074dc5b6")
}
fn karn() -> CardIndex {
    card_index("a20dd48d-d344-4db1-b0e9-a2b71c3cc9d1")
}
fn mycosynth_lattice() -> CardIndex {
    card_index("ae1f2ab5-c6a5-4d49-a746-3cb4668bf805")
}
fn force_of_negation() -> CardIndex {
    card_index("ac2173f9-f223-440a-9231-fd98762bdc6f")
}
fn brainstorm() -> CardIndex {
    card_index("36cd2364-d113-47d1-b2c4-b088d9eb88dd")
}
fn double_major() -> CardIndex {
    card_index("ece44a82-dcf0-4439-bdd9-a09c99a6f159")
}
fn loran() -> CardIndex {
    card_index("b3d81980-76f2-44e2-b1c9-01e30c726312")
}
fn general_tazri() -> CardIndex {
    card_index("b0f19cba-1339-4518-8320-d7b1dcaf2eb0")
}
fn harabaz_druid() -> CardIndex {
    card_index("ead985ec-f29f-4a3b-b8b1-061142cc5bd1")
}
fn halimar_excavator() -> CardIndex {
    card_index("fd3e37c9-93bf-4f3e-a279-22afbffd8d43")
}

/// Every object of `card` a seat controls in `zone`, in zone order.
fn objects_in(
    engine: &Engine<RegistryLookup>,
    zone: ZoneLocation,
    seat: PlayerId,
    card: CardIndex,
) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(zone)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Taps one untapped `card` the seat controls for mana, answering the colour
/// choice a dual or any-colour source asks for.
#[track_caller]
fn tap_for(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    color: Option<ManaColor>,
) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority to tap a source, got {:?}",
            engine.pending()
        )
    };
    let is_it = |engine: &Engine<RegistryLookup>, id: ObjectId| {
        engine
            .state()
            .object(id)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
    };
    // A basic taps through the intrinsic shortcut; a dual has no single
    // colour to shortcut to and taps through its printed ability instead.
    if let Some(source) = legal
        .mana_abilities
        .iter()
        .copied()
        .find(|id| is_it(engine, *id))
    {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .expect("mana ability activates");
    } else {
        let (source, ability_index) = legal
            .abilities
            .iter()
            .copied()
            .find(|(id, _)| is_it(engine, *id))
            .expect("an untapped source of that card is available");
        engine
            .apply(
                seat,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                },
            )
            .expect("printed mana ability activates");
    }
    if let Pending::ChooseColor { player, options } = engine.pending().clone() {
        let want = color.expect("this source asks for a colour");
        assert!(options.contains(&want), "{want:?} is not among {options:?}");
        engine
            .apply(player, PlayerAction::ChooseColor(want))
            .expect("colour chosen");
    }
}

/// Taps every mana ability the seat currently has.
#[track_caller]
fn tap_everything(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .expect("mana ability activates");
    }
}

/// Casts the one copy of `card` in the seat's hand.
#[track_caller]
fn cast_from_hand(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) {
    let obj = objects_in(engine, ZoneLocation::Hand(seat), seat, card)
        .first()
        .copied()
        .expect("the card is in hand");
    engine
        .apply(seat, PlayerAction::CastSpell { card: obj })
        .expect("the spell casts");
}

/// Passes priority (and declares nothing in combat) until `pred` holds,
/// answering an optional trigger with "no" so an unrelated may-ability
/// cannot stall the walk.
#[track_caller]
fn settle(engine: &mut Engine<RegistryLookup>, pred: impl Fn(&Engine<RegistryLookup>) -> bool) {
    for _ in 0..80 {
        if pred(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            Pending::ChooseTargets {
                player,
                options,
                min,
                ..
            } => {
                let objects = if min == 0 {
                    vec![]
                } else {
                    options.first().copied().into_iter().collect()
                };
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects })
                    .unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected while settling: {other:?}"),
        }
    }
    panic!("condition never reached");
}

/// Doubling Season's second line is about counters, and a planeswalker's
/// starting loyalty *is* counters (CR 306.5b + 614.16) — so Karn enters with
/// ten, not five. This was the last rider the card's header still claimed
/// was missing; it was written before planeswalkers existed at all.
#[test]
fn doubling_season_doubles_a_walkers_starting_loyalty() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(21, island())
        .battlefield(
            0,
            &[doubling_season(), island(), island(), island(), island()],
        )
        .hand(0, &[karn()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_everything(&mut engine, p0);
    cast_from_hand(&mut engine, p0, karn());
    settle(&mut engine, |e| {
        !objects_in(e, ZoneLocation::Battlefield, p0, karn()).is_empty()
    });

    let walker = objects_in(&engine, ZoneLocation::Battlefield, p0, karn())[0];
    assert_eq!(
        engine
            .state()
            .object(walker)
            .expect("walker")
            .counters
            .get(baylee_cards_dsl::CounterKind::Loyalty),
        10,
        "Karn's printed five loyalty was doubled on the way in"
    );
}

/// "Players may spend mana as though it were mana of any color": five
/// Islands pay for `{4}{G}` only while the Lattice is out. The control half
/// is the point — without it the same hand is uncastable, so the test
/// cannot pass by accident.
#[test]
fn mycosynth_lattice_lets_blue_mana_pay_a_green_cost() {
    let p0 = PlayerId::new(0);
    let lands = [island(), island(), island(), island(), island()];

    let castable_with = |lattice: bool| {
        let mut battlefield = lands.to_vec();
        if lattice {
            battlefield.push(mycosynth_lattice());
        }
        let mut engine = Duel::new(22, island())
            .battlefield(0, &battlefield)
            .hand(0, &[doubling_season()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        tap_everything(&mut engine, p0);
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority")
        };
        let season = objects_in(&engine, ZoneLocation::Hand(p0), p0, doubling_season())[0];
        (engine, legal.castable.contains(&season))
    };

    let (_, without) = castable_with(false);
    assert!(
        !without,
        "five Islands do not pay {{4}}{{G}} on their own — if they did, the \
         Lattice half of this test would prove nothing"
    );

    let (mut engine, with) = castable_with(true);
    assert!(with, "with the Lattice out, blue mana pays the green pip");

    cast_from_hand(&mut engine, p0, doubling_season());
    settle(&mut engine, |e| {
        !objects_in(e, ZoneLocation::Battlefield, p0, doubling_season()).is_empty()
    });
}

/// "Counter target noncreature spell. If that spell is countered this way,
/// exile it instead of putting it into its owner's graveyard." The
/// destination is the whole point of the card over Counterspell, and it is
/// what the header claimed was missing.
#[test]
fn force_of_negation_exiles_the_spell_it_counters() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(23, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[force_of_negation()])
        .battlefield(1, &[island()])
        .hand(1, &[brainstorm()])
        .start();
    keep_mulligans(&mut engine);
    // Turn one belongs to the other seat; walk through its combat to the
    // turn where Force of Negation is free to pitch.
    settle(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });

    tap_everything(&mut engine, p1);
    cast_from_hand(&mut engine, p1, brainstorm());
    let spell = objects_in(&engine, ZoneLocation::Stack, p1, brainstorm())[0];

    // Priority comes back to the non-active player with the spell on the
    // stack; that is the window Force of Negation is printed for.
    settle(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_everything(&mut engine, p0);
    cast_from_hand(&mut engine, p0, force_of_negation());
    // Three Islands are up, so the wizard offers the printed cost beside the
    // pitch; the rider under test is the destination, not the discount.
    if let Pending::ChooseCastMode { player, options } = engine.pending().clone() {
        let normal = options
            .iter()
            .position(|o| matches!(o.kind, crate::choice::CastModeKind::Normal))
            .expect("the printed cost is affordable");
        engine
            .apply(player, PlayerAction::ChooseMode(normal))
            .expect("mode chosen");
    }
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&spell), "the Brainstorm is targetable");
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("target chosen");

    settle(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Stack).is_empty()
    });

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&spell),
        "the countered spell was exiled"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .contains(&spell),
        "and did not reach its owner's graveyard on the way"
    );
}

/// "Copy target creature spell you control, except it isn't legendary if the
/// spell is legendary." Two Lorans on the stack, exactly one of them
/// legendary — which is what stops the legend rule from eating the copy the
/// moment both resolve.
#[test]
fn a_copy_made_by_double_major_is_not_legendary() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(24, island())
        .battlefield(0, &[plains(), island(), forest(), island(), forest()])
        .hand(0, &[loran(), double_major()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{W} for Loran, held on the stack; then {G}{U} for Double Major.
    tap_for(&mut engine, p0, plains(), None);
    tap_for(&mut engine, p0, island(), None);
    tap_for(&mut engine, p0, forest(), None);
    cast_from_hand(&mut engine, p0, loran());
    tap_for(&mut engine, p0, island(), None);
    tap_for(&mut engine, p0, forest(), None);
    cast_from_hand(&mut engine, p0, double_major());

    let original = objects_in(&engine, ZoneLocation::Stack, p0, loran())[0];
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&original), "the Loran spell is targetable");
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![original],
            },
        )
        .expect("target chosen");

    settle(&mut engine, |e| {
        objects_in(e, ZoneLocation::Stack, p0, loran()).len() == 2
    });

    let legendary: Vec<bool> = objects_in(&engine, ZoneLocation::Stack, p0, loran())
        .iter()
        .map(|id| {
            engine
                .state()
                .object(*id)
                .expect("spell")
                .characteristics()
                .supertypes
                .contains(baylee_core::types::SupertypeSet::LEGENDARY)
        })
        .collect();
    assert_eq!(
        legendary.iter().filter(|l| **l).count(),
        1,
        "the original is still legendary and the copy is not: {legendary:?}"
    );
}

/// "{W}{U}{B}{R}{G}: Ally creatures you control get +X/+X until end of turn,
/// where X is the number of colors among those creatures." Tazri (white),
/// Harabaz Druid (green) and Halimar Excavator (blue) are three colours, so
/// the Druid's printed 0/1 becomes 3/4.
#[test]
fn general_tazri_pumps_allies_by_the_colours_among_them() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(25, island())
        .battlefield(
            0,
            &[
                general_tazri(),
                harabaz_druid(),
                halimar_excavator(),
                plains(),
                island(),
                swamp(),
                forest(),
                badlands(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let druid = objects_in(&engine, ZoneLocation::Battlefield, p0, harabaz_druid())[0];
    assert_eq!(pt(&engine, druid), (0, 1), "the Druid starts at its print");

    tap_for(&mut engine, p0, plains(), None);
    tap_for(&mut engine, p0, island(), None);
    tap_for(&mut engine, p0, swamp(), None);
    tap_for(&mut engine, p0, forest(), None);
    tap_for(&mut engine, p0, badlands(), Some(ManaColor::Red));

    let tazri = objects_in(&engine, ZoneLocation::Battlefield, p0, general_tazri())[0];
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority")
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == tazri)
        .expect("Tazri's five-colour pump is offered once the mana is up");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the ability activates");

    settle(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Stack).is_empty()
    });
    assert_eq!(
        pt(&engine, druid),
        (3, 4),
        "three colours among the Allies, so +3/+3"
    );
}

/// CR 305.6 gives a land one mana ability per basic land type. The engine's
/// shortcut collapsed them into one and picked by a fixed scan order, so
/// Badlands only ever made black and Godless Shrine only ever made white —
/// half of every dual in the pool was unreachable, quietly.
#[test]
fn a_dual_land_taps_for_either_of_its_colours() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(26, island())
        .battlefield(0, &[badlands(), godless_shrine()])
        .start();
    // The Shrine asks its shock question before the opening hands are kept.
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            _ => break,
        }
    }
    settle(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });

    // The colour the old scan order would never have offered, on both cards.
    tap_for(&mut engine, p0, badlands(), Some(ManaColor::Red));
    tap_for(&mut engine, p0, godless_shrine(), Some(ManaColor::Black));

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "Badlands made red");
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "and the Shrine made black"
    );
}

/// A mana ability resolves off the stack (CR 605.3b), but a colour choice
/// suspends it like any other resolution — and the completion path used to
/// treat the *source permanent* as a resolving spell. `finalize_spell`
/// untapped it, so tapping Badlands for red left it untapped and ready to
/// tap again: unbounded mana out of one land.
#[test]
fn choosing_a_colour_does_not_untap_the_land_that_made_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(27, island())
        .battlefield(0, &[badlands()])
        .start();
    keep_mulligans(&mut engine);
    settle(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });

    let land = objects_in(&engine, ZoneLocation::Battlefield, p0, badlands())[0];
    tap_for(&mut engine, p0, badlands(), Some(ManaColor::Red));

    assert!(
        engine
            .state()
            .object(land)
            .expect("the land is still there")
            .status
            .contains(Status::TAPPED),
        "paying {{T}} left it tapped"
    );
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "activating a mana ability keeps priority");
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == land) && !legal.mana_abilities.contains(&land),
        "and it cannot be tapped a second time"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "exactly one mana came out of one land"
    );
}
