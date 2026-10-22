//! `cards/creatures/mv_3/hurloon_shaman.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{HURLOON_SHAMAN, LIGHTNING_BOLT};

/// `Hurloon Shaman` ({1}{R}{R}, Creature — Minotaur Shaman 2/3) makes each
/// player sacrifice a land of their choice when it dies.
#[test]
fn hurloon_shaman_makes_each_player_sacrifice_land_on_death() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[HURLOON_SHAMAN, mountain(), mountain()])
        .battlefield(1, &[plains()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shaman = on_battlefield(&engine, p0, HURLOON_SHAMAN).unwrap();
    let land0 = on_battlefield(&engine, p0, mountain()).unwrap();
    let land1 = on_battlefield(&engine, p1, plains()).unwrap();
    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, shaman);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land0],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![land1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, HURLOON_SHAMAN).is_some());
    assert!(in_graveyard(&engine, p0, mountain()).is_some());
    assert!(in_graveyard(&engine, p1, plains()).is_some());
}
