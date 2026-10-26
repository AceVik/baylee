//! `cards/creatures/mv_4/ivy_seer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn ivy_seer() -> CardIndex {
    card_index("f61591d5-0f2a-48cd-a091-863d2e93e019")
}

/// Ivy Seer (f61591d5-0f2a-48cd-a091-863d2e93e019) — Partial: casts as a 1/1
/// green Elf Wizard for {3}{G}; the variable hand-reveal activated ability is
/// omitted and not offered.
#[test]
fn ivy_seer_casts_as_one_one_and_does_not_offer_unimplemented_reveal_ability() {
    let mut e = lk::game(SEED, &[], &[], &[ivy_seer(), forest()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 4)]);
    cast_with_floating(&mut e, lk::P0, ivy_seer());
    lk::settle(&mut e);

    let seer = lk::obj(&e, lk::P0, ivy_seer());
    assert_eq!(pt(&e, seer), (1, 1));
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    lk::float(&mut e, lk::P0, &[(ManaColor::Green, 3)]);
    assert!(!lk::offered(&e, ivy_seer(), lk::P0, 0));
}
