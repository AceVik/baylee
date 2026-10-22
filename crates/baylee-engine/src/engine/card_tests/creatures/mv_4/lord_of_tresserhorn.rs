//! `cards/creatures/mv_4/lord_of_tresserhorn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::life;
use baylee_core::generated::index::{HILL_GIANT, TERROR};

fn lord_of_tresserhorn() -> CardIndex {
    card_index("283e7a23-ae25-48d1-abb5-cd021f66cc29")
}

/// Lord of Tresserhorn ({1}{U}{B}{R} 10/4 Legendary Creature — Zombie):
/// - "When Lord of Tresserhorn enters, you lose 2 life, you sacrifice two creatures, and target opponent draws two cards."
/// - "{B}: Regenerate Lord of Tresserhorn."
#[test]
fn lord_of_tresserhorn_etb_loses_life_sacrifices_two_creatures_opponent_draws_and_regenerates() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                island(),
                swamp(),
                mountain(),
                swamp(),
                swamp(),
                grizzly_bears(),
                HILL_GIANT,
            ],
        )
        .hand(0, &[lord_of_tresserhorn()])
        .hand(1, &[TERROR])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let life_before = life(&engine, p0);
    let opp_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, lord_of_tresserhorn());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();

    // First creature sacrifice.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();

    // Second creature sacrifice.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let lord = on_battlefield(&engine, p0, lord_of_tresserhorn()).unwrap();
    assert_eq!(pt(&engine, lord), (10, 4));
    assert_eq!(life(&engine, p0), life_before - 2);
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(in_graveyard(&engine, p0, HILL_GIANT).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        opp_hand_before + 2
    );

    // Activate {B}: Regenerate Lord of Tresserhorn, then verify lethal damage is replaced by regeneration.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: lord,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(lord).unwrap().regeneration_shields, 1);
}
