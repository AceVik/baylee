//! `cards/instants/mv_1/great_defender.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Great Defender: "Target creature gets +0/+X until end of turn, where X is its mana value."
/// A 1/2 Ondu Cleric with mana value 2 is targeted by Great Defender.
/// After the instant resolves, its toughness increases by 2 to 4 while its power remains 1.
#[test]
fn great_defender_boosts_toughness_by_target_mana_value() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[plains(), ondu_cleric()])
        .hand(0, &[great_defender()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("Ondu Cleric deployed");
    assert_eq!(pt(&engine, cleric), (1, 1));

    tap_all_mana(&mut engine, p0);
    let gd = in_hand(&engine, p0, great_defender()).expect("Great Defender in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: gd })
        .expect("one Plains pays {W}");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&cleric));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, cleric),
        (1, 3),
        "Ondu Cleric has mana value 2 and gets +0/+2"
    );
}
