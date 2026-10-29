//! Rogue Kavu — {1}{R} — Creature — Kavu
//! Oracle: Whenever this creature attacks alone, it gets +2/+0 until end of turn.

use super::*;
use crate::engine::card_tests::legends_kit as lk;

#[test]
fn rogue_kavu_gets_plus_two_power_only_when_it_attacks_alone() {
    let kavu = card_index("17df95b2-49ef-43ca-b3c8-66a270f457a7");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for alone in [false, true] {
        let mut e = lk::game(SEED, &[kavu, grizzly_bears()], &[], &[], &[]);
        let kavu_id = lk::obj(&e, p0, kavu);
        let other_id = lk::obj(&e, p0, grizzly_bears());
        let attackers = if alone {
            vec![kavu_id]
        } else {
            vec![kavu_id, other_id]
        };
        lk::attack(&mut e, &attackers);
        lk::settle(&mut e);
        assert_eq!(pt(&e, kavu_id), if alone { (3, 1) } else { (1, 1) });
        reach_their_main_phase(&mut e, p1);
        assert_eq!(pt(&e, kavu_id), (1, 1));
    }
}
