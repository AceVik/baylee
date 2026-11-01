//! Entry counter and supported body, played.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
#[test]
fn enters_with_one_counter_and_no_underpriced_regeneration() {
    let subject = card_index("88782913-bfba-4300-b26c-405e6a45537c");
    let mut e = lk::game(SEED, &[], &[], &[subject], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Black, 10)]);
    cast_with_floating(&mut e, lk::P0, subject);
    lk::settle(&mut e);
    let body = lk::obj(&e, lk::P0, subject);
    assert_eq!(pt(&e, body), (1, 1));
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority expected")
    };
    assert!(!legal.abilities.iter().any(|(source, _)| *source == body));
    reach_their_main_phase(&mut e, lk::P1);
    reach_their_main_phase(&mut e, lk::P0);
    lk::attackers_offered(&mut e);
    lk::declare(&mut e, &[body]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 19);
}
