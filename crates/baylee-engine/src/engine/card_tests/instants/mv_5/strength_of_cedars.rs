//! `cards/instants/mv_5/strength_of_cedars.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Strength of Cedars: "Target creature gets +X/+X until end of turn, where X is the number of lands you control."
/// Cast with five Forests on the battlefield targeting a 1/1 Llanowar Elves, X is evaluated as five.
/// Upon resolution, the creature receives +5/+5 and its power and toughness become 6/6.
#[test]
fn strength_of_cedars_pumps_target_creature_by_lands_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(103, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[strength_of_cedars()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert_eq!(pt(&engine, elves), (1, 1));

    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, strength_of_cedars()).expect("Strength of Cedars in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "Elves is a legal target");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, elves),
        (6, 6),
        "five Forests give +5/+5 to the 1/1 Elves"
    );
}
