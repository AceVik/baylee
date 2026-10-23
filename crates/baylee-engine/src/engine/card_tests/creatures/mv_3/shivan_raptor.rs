#[allow(clippy::wildcard_imports)]
use super::*;

fn shivan_raptor() -> CardIndex {
    card_index("bc01d36b-dea3-47d4-afdf-0dbbba7ca7b2")
}

use crate::engine::card_tests::legends_kit as lk;

#[test]
fn shivan_raptor_echo_is_due_once_next_upkeep_and_requires_payment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for pay in [false, true] {
        let mut e = lk::game(SEED, &[], &[], &[shivan_raptor()], &[]);
        lk::float(&mut e, p0, &[(ManaColor::Red, 3)]);
        cast_with_floating(&mut e, p0, shivan_raptor());
        lk::settle(&mut e);
        reach_their_main_phase(&mut e, p1);
        assert!(on_battlefield(&e, p0, shivan_raptor()).is_some());
        pass_until(&mut e, |e| {
            e.state().turn.active == p0 && e.state().turn.step == crate::turn::Step::Upkeep
        });
        lk::float(&mut e, p0, &[(ManaColor::Red, 3)]);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        lk::yes_no(&mut e, pay);
        reach_main_phase(&mut e, p0);
        assert_eq!(on_battlefield(&e, p0, shivan_raptor()).is_some(), pay);
        if pay {
            reach_their_main_phase(&mut e, p1);
            reach_their_main_phase(&mut e, p0);
            assert!(
                on_battlefield(&e, p0, shivan_raptor()).is_some(),
                "no second echo price"
            );
        }
    }
}

#[test]
fn shivan_raptor_attacks_immediately_and_kills_a_blocker_before_normal_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[], &[grizzly_bears()], &[shivan_raptor()], &[]);
    lk::float(&mut e, p0, &[(ManaColor::Red, 3)]);
    lk::cast_at(&mut e, p0, shivan_raptor(), &[], &[]);
    let raptor = lk::obj(&e, p0, shivan_raptor());
    let bear = lk::obj(&e, p1, grizzly_bears());
    lk::attack(&mut e, &[raptor]);
    lk::block(&mut e, &[(bear, raptor)]);
    lk::through_combat(&mut e);
    assert!(in_graveyard(&e, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&e, p0, shivan_raptor()).is_some());
    assert_eq!(lk::damage_on(&e, raptor), 0);
}
