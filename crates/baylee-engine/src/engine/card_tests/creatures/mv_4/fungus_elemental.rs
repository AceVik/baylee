//! `cards/creatures/mv_4/fungus_elemental.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::FUNGUS_ELEMENTAL;

/// `Fungus Elemental` ({3}{G}, Creature — Fungus Elemental 3/3) sacrifices a
/// Forest for `{G}` on the turn it entered to get a +2/+2 counter.
#[test]
fn fungus_elemental_sacrifices_forest_on_arrival_turn_for_plus_2_plus_2_counter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[FUNGUS_ELEMENTAL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, FUNGUS_ELEMENTAL);
    pass_until(&mut engine, stack_is_empty);

    let elemental = on_battlefield(&engine, p0, FUNGUS_ELEMENTAL).unwrap();
    assert_eq!(pt(&engine, elemental), (3, 3));

    let sac_forest = on_battlefield(&engine, p0, forest()).unwrap();
    activate(&mut engine, p0, FUNGUS_ELEMENTAL, 0);
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![sac_forest],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elemental), (5, 5));
}
