//! `cards/sorceries/mv_2/hunger_of_the_nim.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hunger of the Nim: "Target creature gets +1/+0 until end of turn for each artifact you control."
/// Cast with two artifacts on the battlefield targeting a 1/1 Llanowar Elves, the pump counts both artifacts.
/// Upon resolution, the creature receives +2/+0 and its power and toughness become 3/1.
#[test]
fn hunger_of_the_nim_pumps_target_creature_per_artifact_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(135, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                myr_retriever(),
                myr_retriever(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[hunger_of_the_nim()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("Elves deployed");
    assert_eq!(pt(&engine, elves), (1, 1));

    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, hunger_of_the_nim()).expect("Hunger of the Nim in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elves), "Elves is a legal target creature");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elves), (3, 1), "two artifacts give +2/+0");
}
