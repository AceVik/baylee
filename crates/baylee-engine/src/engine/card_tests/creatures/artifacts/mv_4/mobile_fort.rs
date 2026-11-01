#[allow(clippy::wildcard_imports)]
use super::*;

fn mobile_fort() -> CardIndex {
    card_index("0547e426-e23e-4c60-ba6b-cfd4b5ff8485")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn mobile_fort_attacks_only_after_activation_once_each_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[mobile_fort()], &[], &[], &[]);
    let fort = lk::obj(&e, p0, mobile_fort());
    lk::float(&mut e, p0, &[(ManaColor::Colorless, 6)]);
    activate(&mut e, p0, mobile_fort(), 0);
    lk::settle(&mut e);
    assert_eq!(pt(&e, fort), (3, 5));
    assert!(!lk::offered(&e, mobile_fort(), p0, 0));
    assert!(lk::attackers_offered(&mut e).contains(&fort));
    lk::declare(&mut e, &[fort]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 17);
    reach_their_main_phase(&mut e, p1);
    assert_eq!(pt(&e, fort), (0, 6));
    reach_their_main_phase(&mut e, p0);
    lk::float(&mut e, p0, &[(ManaColor::Colorless, 3)]);
    assert!(lk::offered(&e, mobile_fort(), p0, 0));
    assert!(!lk::attackers_offered(&mut e).contains(&fort));
}
