//! Reviewed essence bottle behavior.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn essence_bottle() -> CardIndex {
    card_index("76f839b0-1d4c-4ad9-b44f-a8a13d6d109f")
}

#[test]
fn essence_bottle_stores_a_counter_but_offers_no_costless_redemption() {
    let mut e = lk::game(SEED, &[essence_bottle()], &[], &[], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Colorless, 3)]);
    lk::use_ability(&mut e, lk::P0, essence_bottle(), 0, &[], &[]);
    let bottle = lk::obj(&e, lk::P0, essence_bottle());
    assert_eq!(
        e.state()
            .object(bottle)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::ELIXIR),
        1
    );
    assert!(lk::tapped(&e, bottle));
    assert_eq!(lk::pool_total(&e, lk::P0), 0);
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    assert!(!lk::offered(&e, essence_bottle(), lk::P0, 1));
    assert_eq!(lk::life(&e, lk::P0), 20);
}
