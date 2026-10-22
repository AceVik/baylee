//! `cards/creatures/mv_2/orcish_settlers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::ORCISH_SETTLERS;

/// `Orcish Settlers` ({1}{R}, Creature — Orc 1/1) pays `{X}{X}{R}, {T},
/// Sacrifice this creature` to destroy X target lands.
#[test]
fn orcish_settlers_sacrifices_to_destroy_x_target_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                ORCISH_SETTLERS,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[plains(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land1 = on_battlefield(&engine, p1, plains()).unwrap();
    let land2 = on_battlefield(&engine, p1, island()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ORCISH_SETTLERS, 0);
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land1, land2],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, ORCISH_SETTLERS).is_some());
    assert!(in_graveyard(&engine, p1, plains()).is_some());
    assert!(in_graveyard(&engine, p1, island()).is_some());
}
