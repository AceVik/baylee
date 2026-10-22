//! `cards/enchantments/mv_3/dense_foliage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{DENSE_FOLIAGE, GRIZZLY_BEARS, LIGHTNING_BOLT};

/// `Dense Foliage` ({2}{G}, Enchantment) prevents creatures from being the
/// targets of spells.
#[test]
fn dense_foliage_prevents_creatures_from_being_targeted_by_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[GRIZZLY_BEARS, forest(), forest(), forest()])
        .hand(0, &[DENSE_FOLIAGE, LIGHTNING_BOLT, mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, DENSE_FOLIAGE);
    pass_until(&mut engine, stack_is_empty);

    play_land(&mut engine, p0, mountain());
    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears],
                    players: vec![],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
}
