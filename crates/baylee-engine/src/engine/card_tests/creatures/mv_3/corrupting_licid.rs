//! The supported creature body is played; omitted abilities are not invented.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

#[test]
fn supported_body_attacks_without_an_invented_activation() {
    let subject = card_index("503ddd47-4b0c-4a2a-b77a-fb321044b438");
    let mut e = lk::game(SEED, &[subject], &[], &[], &[]);
    let body = lk::obj(&e, lk::P0, subject);
    for color in ManaColor::ALL {
        lk::float(&mut e, lk::P0, &[(color, 10)]);
    }
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority expected")
    };
    assert!(!legal.abilities.iter().any(|(source, _)| *source == body));
    lk::attackers_offered(&mut e);
    lk::declare(&mut e, &[body]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);
}
