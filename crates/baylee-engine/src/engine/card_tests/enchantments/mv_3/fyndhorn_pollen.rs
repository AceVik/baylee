//! `cards/enchantments/mv_3/fyndhorn_pollen.rs`, played.
//!
//! Oracle:
//! - "Cumulative upkeep {1}"
//! - "All creatures get -1/-0."
//! - "{1}{G}: All creatures get -1/-0 until end of turn."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn fyndhorn_pollen() -> CardIndex {
    card_index("2f18967d-c2cd-449d-a288-df6343672743")
}

#[test]
fn fyndhorn_pollen_debuffs_all_creatures_statically_and_when_activated() {
    let mut engine = lk::game_with(
        SEED,
        &[fyndhorn_pollen(), index::GRIZZLY_BEARS],
        &[],
        &[],
        &[],
        |e| {
            lk::float(e, P0, &[(ManaColor::Colorless, 1)]);
            lk::yes_no(e, true);
        },
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .power,
        Some(1),
        "static ability gives all creatures -1/-0"
    );

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
    );
    activate(&mut engine, P0, fyndhorn_pollen(), 2);
    lk::settle(&mut engine);

    assert_eq!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .power,
        Some(0),
        "activated ability gives all creatures an additional -1/-0 until end of turn"
    );
}
