//! The remaining Partial body is cast; unsupported approximations are absent.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

#[test]
fn unsupported_approximation_is_not_an_ability() {
    let card = card_index("cad0f865-62e0-47ba-84ae-8e97cb52bbd1");
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(1, &[baylee_core::generated::index::GRIZZLY_BEARS])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    lk::float(&mut e, p0, &ManaColor::ALL.map(|color| (color, 10)));
    cast_with_floating(&mut e, p0, card);
    pass_until(&mut e, stack_is_empty);
    let creature = on_battlefield(&e, p0, card).expect("the remaining body is cast");
    reach_their_main_phase(&mut e, PlayerId::new(1));
    reach_their_main_phase(&mut e, p0);
    lk::float(&mut e, p0, &ManaColor::ALL.map(|color| (color, 10)));
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority after resolving the creature");
    };
    assert!(legal.abilities.iter().all(|(id, _)| *id != creature));
}
