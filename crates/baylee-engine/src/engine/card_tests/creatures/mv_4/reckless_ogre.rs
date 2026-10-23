#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
fn reckless_ogre() -> CardIndex {
    card_index("fe455177-d9e5-4893-ad87-ebbba5e2fa45")
}
#[test]
fn reckless_ogre_gets_the_bonus_only_when_it_attacks_alone_and_only_for_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for alone in [false, true] {
        let mut e = lk::game(SEED, &[reckless_ogre(), gray_ogre()], &[], &[], &[]);
        let ogre = lk::obj(&e, p0, reckless_ogre());
        let other = lk::obj(&e, p0, gray_ogre());
        let attackers = if alone { vec![ogre] } else { vec![ogre, other] };
        lk::attack(&mut e, &attackers);
        lk::settle(&mut e);
        assert_eq!(pt(&e, ogre), if alone { (6, 2) } else { (3, 2) });
        reach_their_main_phase(&mut e, p1);
        assert_eq!(pt(&e, ogre), (3, 2));
    }
}
