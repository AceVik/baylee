#[allow(clippy::wildcard_imports)]
use super::*;

fn pouncing_jaguar() -> CardIndex {
    card_index("70a2b5ea-dae4-46e8-87eb-8c8d8dfb242b")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn pouncing_jaguar_echo_is_due_once_next_upkeep_and_requires_payment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for pay in [false, true] {
        let mut e = lk::game(SEED, &[], &[], &[pouncing_jaguar()], &[]);
        lk::float(&mut e, p0, &[(ManaColor::Green, 1)]);
        cast_with_floating(&mut e, p0, pouncing_jaguar());
        lk::settle(&mut e);
        reach_their_main_phase(&mut e, p1);
        assert!(on_battlefield(&e, p0, pouncing_jaguar()).is_some());
        pass_until(&mut e, |e| {
            e.state().turn.active == p0 && e.state().turn.step == crate::turn::Step::Upkeep
        });
        lk::float(&mut e, p0, &[(ManaColor::Green, 1)]);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        lk::yes_no(&mut e, pay);
        reach_main_phase(&mut e, p0);
        assert_eq!(on_battlefield(&e, p0, pouncing_jaguar()).is_some(), pay);
        if pay {
            reach_their_main_phase(&mut e, p1);
            reach_their_main_phase(&mut e, p0);
            assert!(
                on_battlefield(&e, p0, pouncing_jaguar()).is_some(),
                "no second echo price"
            );
        }
    }
}
