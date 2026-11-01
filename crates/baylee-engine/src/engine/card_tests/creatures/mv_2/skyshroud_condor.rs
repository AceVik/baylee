//! Skyshroud Condor cannot be the first spell of a turn.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;
fn skyshroud_condor() -> CardIndex {
    card_index("c7c4c67e-c4db-416b-b2e8-f0e59aa80bba")
}

#[test]
fn condor_requires_an_earlier_spell_and_forgets_it_next_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = lk::game(
        SEED,
        &[],
        &[],
        &[skyshroud_condor(), skyshroud_condor(), index::GRIZZLY_BEARS],
        &[],
    );
    lk::float(&mut e, p0, &[(ManaColor::Blue, 10), (ManaColor::Green, 10)]);
    let first = in_hand(&e, p0, skyshroud_condor()).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority");
    };
    assert!(!legal.castable.contains(&first) && !legal.payable.contains(&first));
    assert!(
        e.apply(p0, PlayerAction::CastSpell { card: first })
            .is_err()
    );
    cast_with_floating(&mut e, p0, index::GRIZZLY_BEARS);
    lk::settle(&mut e);
    cast_with_floating(&mut e, p0, skyshroud_condor());
    lk::settle(&mut e);
    assert!(on_battlefield(&e, p0, skyshroud_condor()).is_some());
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    lk::float(&mut e, p0, &[(ManaColor::Blue, 10)]);
    let remaining = in_hand(&e, p0, skyshroud_condor()).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority");
    };
    assert!(!legal.castable.contains(&remaining) && !legal.payable.contains(&remaining));
}
