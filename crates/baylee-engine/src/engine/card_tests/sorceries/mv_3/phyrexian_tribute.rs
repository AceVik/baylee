//! `cards/sorceries/mv_3/phyrexian_tribute.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "c06b993b-a36a-412a-a732-d678a4179791"
fn phyrexian_tribute() -> CardIndex {
    card_index("c06b993b-a36a-412a-a732-d678a4179791")
}

#[test]
fn phyrexian_tribute_sacrifices_two_creatures_and_destroys_target_artifact() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[index::JAYEMDAE_TOME],
        &[phyrexian_tribute()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let tome = lk::obj(&engine, P1, index::JAYEMDAE_TOME);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P0, phyrexian_tribute());
    lk::aim(&mut engine, &[tome], &[]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert!(on_battlefield(&engine, P0, index::HILL_GIANT).is_none());
    assert!(on_battlefield(&engine, P1, index::JAYEMDAE_TOME).is_none());
}
