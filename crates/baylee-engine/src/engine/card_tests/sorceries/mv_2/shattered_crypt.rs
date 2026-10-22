//! `cards/sorceries/mv_2/shattered_crypt.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, SHATTERED_CRYPT};

/// `Shattered Crypt` ({X}{B}{B}, Sorcery) returns X target creature cards from
/// your graveyard to your hand and you lose X life.
#[test]
fn shattered_crypt_returns_x_creatures_from_graveyard_and_loses_x_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                GRIZZLY_BEARS,
                HILL_GIANT,
            ],
        )
        .hand(0, &[SHATTERED_CRYPT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    let _ = engine.state.move_object(
        bears,
        ZoneLocation::Graveyard(p0),
        crate::zone::ZonePosition::Top,
        Cause::Effect,
    );
    let _ = engine.state.move_object(
        giant,
        ZoneLocation::Graveyard(p0),
        crate::zone::ZonePosition::Top,
        Cause::Effect,
    );
    engine.refresh_offer();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, SHATTERED_CRYPT);
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears, giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, p0, HILL_GIANT).is_some());
    assert_eq!(engine.state().players[0].life, 18);
}
