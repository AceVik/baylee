//! `cards/creatures/mv_3/royal_trooper.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn royal_trooper() -> CardIndex {
    card_index("e31510d5-b74e-4a8d-94e8-8bd098cac3c7")
}

/// Royal Trooper ({2}{W} 2/2 Human Soldier):
/// "Whenever this creature blocks, it gets +2/+2 until end of turn."
#[test]
fn royal_trooper_gets_plus_two_plus_two_only_when_blocking_until_end_of_turn() {
    let mut e = lk::game(SEED, &[royal_trooper()], &[index::HILL_GIANT], &[], &[]);
    let trooper = lk::obj(&e, lk::P0, royal_trooper());
    let giant = lk::obj(&e, lk::P1, index::HILL_GIANT);

    let _ = lk::attack(&mut e, &[trooper]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(pt(&e, trooper), (2, 2));

    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    reach_their_main_phase(&mut e, lk::P1);
    let _ = lk::attack(&mut e, &[giant]);
    lk::block(&mut e, &[(trooper, giant)]);
    lk::settle(&mut e);
    assert_eq!(pt(&e, trooper), (4, 4));

    lk::through_combat(&mut e);
    assert!(in_graveyard(&e, lk::P1, index::HILL_GIANT).is_some());
    assert!(on_battlefield(&e, lk::P0, royal_trooper()).is_some());

    reach_their_main_phase(&mut e, lk::P0);
    assert_eq!(pt(&e, trooper), (2, 2));
}
