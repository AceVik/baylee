//! `cards/creatures/mv_2/harvest_wurm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HARVEST_WURM;

/// `Harvest Wurm` ({1}{G}, Creature — Wurm 3/2, Partial) optionally returns a
/// basic land card from your graveyard to your hand when it enters.
#[test]
fn harvest_wurm_returns_basic_land_from_graveyard_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[HARVEST_WURM, forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gy_forest = hand_to_graveyard(&mut engine, p0, forest());
    cast_from_hand(&mut engine, p0, HARVEST_WURM);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![gy_forest],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, HARVEST_WURM).is_some());
    assert!(in_graveyard(&engine, p0, forest()).is_none());
}
