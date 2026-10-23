//! `cards/sorceries/mv_4/phyrexian_purge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "df032801-fccf-4bff-aa88-d01650b6c81e"
fn phyrexian_purge() -> CardIndex {
    card_index("df032801-fccf-4bff-aa88-d01650b6c81e")
}

#[test]
fn phyrexian_purge_destroys_target_creature() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[phyrexian_purge()],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 2),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
    );
    lk::cast_at(&mut engine, P0, phyrexian_purge(), &[bears], &[]);

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
}

/// "Costs 3 life more to cast": 20 life becomes 17 when it is cast, and with
/// 2 life the spell cannot be cast at all.
#[test]
fn phyrexian_purge_costs_three_life() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[phyrexian_purge()],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 2),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
    );
    lk::cast_at(&mut engine, P0, phyrexian_purge(), &[bears], &[]);
    assert_eq!(lk::life(&engine, P0), 17);

    let mut poor = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[phyrexian_purge()],
        &[],
    );
    poor.dev_state_mut(P0).unwrap().players[0].life = 2;
    lk::float(
        &mut poor,
        P0,
        &[
            (ManaColor::Colorless, 2),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
    );
    assert!(
        !lk::castable(&poor, P0, phyrexian_purge()),
        "2 life cannot pay 3"
    );
}
