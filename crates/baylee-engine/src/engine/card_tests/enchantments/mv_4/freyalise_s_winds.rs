//! `cards/enchantments/mv_4/freyalise_s_winds.rs`, played.
//!
//! Oracle:
//! - "Whenever a permanent becomes tapped, put a wind counter on it."
//! - "If a permanent with a wind counter on it would untap during its controller's untap step, remove all wind counters from it instead."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn freyalises_winds() -> CardIndex {
    card_index("ffeb792a-6df0-46c1-badc-4deb857f7d48")
}

#[test]
fn freyalises_winds_puts_wind_counter_on_permanent_when_tapped() {
    let mut engine = lk::game(SEED, &[freyalises_winds(), index::FOREST], &[], &[], &[]);
    let land = lk::obj(&engine, P0, index::FOREST);

    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(
        engine
            .state()
            .object(land)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::WIND),
        1,
        "tapping a permanent puts a wind counter on it"
    );
}
