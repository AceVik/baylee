//! `cards/instants/mv_2/prismatic_boon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

// oracle_id = "aee5dda1-2181-40d5-9846-85d154e1d5da"
fn prismatic_boon() -> CardIndex {
    card_index("aee5dda1-2181-40d5-9846-85d154e1d5da")
}

#[test]
fn prismatic_boon_casts_with_x_targeting_a_creature() {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[], &[prismatic_boon()], &[]);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, prismatic_boon());
    engine.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P0, prismatic_boon()).is_some());
}
