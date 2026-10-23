//! Marjhan — `{5}{U}{U}` 8/8 Creature — Serpent (Homelands)
//!
//! This creature doesn't untap during your untap step.
//! `{U}{U}`, Sacrifice a creature: Untap this creature. Activate only during
//! your upkeep.
//! This creature can't attack unless defending player controls an Island.
//! `{U}{U}`: This creature gets -1/-0 until end of turn and deals 1 damage to
//! target attacking creature without flying.
//! When you control no Islands, sacrifice this creature.

#[allow(clippy::wildcard_imports)]
use super::*;

use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn marjhan() -> CardIndex {
    card_index("3fdee2ab-7ec6-4fc6-ad99-f04571f94583")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// "{U}{U}: … deals 1 damage to target attacking creature without flying."
/// P1 owns Marjhan and two Islands; P0 attacks with a Bear and a flying
/// Vampire and keeps an Ogre home.
#[test]
fn marjhan_shrinks_to_ping_an_attacking_nonflyer_and_only_that() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                bears(),
                card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b"),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[marjhan(), island(), island()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let bear = on_battlefield(&engine, P0, bears()).unwrap();
    let vampire = on_battlefield(
        &engine,
        P0,
        card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b"),
    )
    .unwrap();
    let ogre = on_battlefield(&engine, P0, gray_ogre()).unwrap();
    let serpent = on_battlefield(&engine, P1, marjhan()).unwrap();

    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bear, Defender::Player(P1)),
                    (vampire, Defender::Player(P1)),
                ],
            },
        )
        .unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, P1);
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: serpent,
                ability_index: 3,
            },
        )
        .unwrap();
    for illegal in [vampire, ogre] {
        assert!(
            engine
                .apply(
                    P1,
                    PlayerAction::ChooseTargets {
                        objects: vec![illegal],
                        players: vec![],
                    },
                )
                .is_err(),
            "a flier and a creature that is not attacking are not targets"
        );
    }
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert_eq!(pt(&engine, serpent), (7, 8), "Marjhan gets -1/-0");
    assert_eq!(engine.state().object(bear).unwrap().damage, 1);
    assert_eq!(engine.state().object(vampire).unwrap().damage, 0);
    assert_eq!(engine.state().object(ogre).unwrap().damage, 0);
}

/// "Can't attack unless defending player controls an Island": no Island
/// across the table, no attack. The same board with an Island on the other
/// side offers it. (The Bears are on the board so that an attack is asked at
/// all.)
#[test]
fn marjhan_cannot_attack_unless_the_defender_controls_an_island() {
    let mut without = Duel::new(SEED, island())
        .battlefield(0, &[marjhan(), island(), bears()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut without);
    pass_until(&mut without, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = without.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    let serpent = on_battlefield(&without, P0, marjhan()).unwrap();
    let bear = on_battlefield(&without, P0, bears()).unwrap();
    assert!(attackers.contains(&bear), "the Bears may attack");
    assert!(
        !attackers.contains(&serpent),
        "the defender has no Island, so Marjhan is not offered"
    );

    let mut with = Duel::new(SEED, island())
        .battlefield(0, &[marjhan(), island(), bears()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut with);
    pass_until(&mut with, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = with.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    let serpent = on_battlefield(&with, P0, marjhan()).unwrap();
    assert!(
        attackers.contains(&serpent),
        "the defender controls an Island"
    );
}

/// A board where Marjhan has attacked and so sits tapped, stopped at P0's
/// next turn after its untap step (the first main phase). P0 has two Islands
/// and a Bears to sacrifice.
fn marjhan_tapped_at_the_next_main_phase() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[marjhan(), island(), island(), bears()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let serpent = on_battlefield(&engine, P0, marjhan()).unwrap();
    let bear = on_battlefield(&engine, P0, bears()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(serpent, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(is_tapped(&engine, serpent), "Marjhan attacked");
    (engine, serpent, bear)
}

/// "Doesn't untap during your untap step", and its untap ability is for the
/// upkeep alone.
#[test]
fn marjhan_does_not_untap_and_cannot_be_untapped_outside_the_upkeep() {
    let (mut engine, serpent, _bear) = marjhan_tapped_at_the_next_main_phase();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        is_tapped(&engine, serpent),
        "it did not untap in its controller's untap step"
    );
    assert!(
        !ability_offered(&engine, serpent, 1),
        "the untap ability is for your upkeep, not your main phase"
    );
}

/// "{U}{U}, Sacrifice a creature: Untap this creature. Activate only during
/// your upkeep": offered in the upkeep, it costs the creature.
#[test]
fn marjhan_untaps_in_its_controllers_upkeep_for_a_sacrificed_creature() {
    let (mut engine, serpent, bear) = marjhan_tapped_at_the_next_main_phase();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(is_tapped(&engine, serpent), "still tapped in the upkeep");
    tap_all_mana(&mut engine, P0);
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: serpent,
                ability_index: 1,
            },
        )
        .expect("offered in your upkeep");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the sacrifice is a choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&bear), "a creature is on the menu");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, serpent), "Marjhan untapped");
    assert!(in_graveyard(&engine, P0, bears()).is_some(), "the cost");
}

/// "When you control no Islands, sacrifice this creature": the opponent's
/// Island is not yours.
#[test]
fn marjhan_is_sacrificed_when_its_controller_controls_no_islands() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[marjhan()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    assert!(on_battlefield(&engine, P0, marjhan()).is_none());
    assert!(in_graveyard(&engine, P0, marjhan()).is_some());

    // The control: with an Island of its own it stays.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[marjhan(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    assert!(on_battlefield(&engine, P0, marjhan()).is_some());
}
