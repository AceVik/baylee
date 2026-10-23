//! The supported creature body is played; omitted abilities are not invented.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

#[test]
fn supported_body_attacks_without_an_invented_activation() {
    let subject = card_index("93d20859-249f-47ae-8f6d-095228e7b6e9");
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
