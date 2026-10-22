//! `cards/artifacts/mv_5/amulet_of_unmaking.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::AMULET_OF_UNMAKING;

#[test]
fn amulet_of_unmaking_exiles_self_and_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                AMULET_OF_UNMAKING,
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let amulet = on_battlefield(&engine, p0, AMULET_OF_UNMAKING).unwrap();
    let land = on_battlefield(&engine, p1, forest()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, AMULET_OF_UNMAKING, 0);
    aim_at(&mut engine, p0, land);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(amulet).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(land).unwrap().zone, Zone::Exile);
}
