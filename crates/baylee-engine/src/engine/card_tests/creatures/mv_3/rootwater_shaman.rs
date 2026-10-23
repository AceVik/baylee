//! The remaining Partial body is cast; unsupported approximations are absent.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

#[test]
fn unsupported_approximation_is_not_an_ability() {
    let card = card_index("8a449915-8bd7-4a9c-abb1-a95852f33695");
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[card, baylee_core::generated::index::CRUSADE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    lk::float(&mut e, p0, &ManaColor::ALL.map(|color| (color, 10)));
    cast_with_floating(&mut e, p0, card);
    pass_until(&mut e, stack_is_empty);
    let creature = on_battlefield(&e, p0, card).expect("the remaining body is cast");
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority after resolving the creature");
    };
    assert!(legal.abilities.iter().all(|(id, _)| *id != creature));
    let p1 = PlayerId::new(1);
    reach_their_main_phase(&mut e, p1);
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    lk::float(&mut e, p0, &[(ManaColor::White, 10)]);
    let enchantment = in_hand(&e, p0, baylee_core::generated::index::CRUSADE).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority");
    };
    assert!(
        !legal.castable.contains(&enchantment) && !legal.payable.contains(&enchantment),
        "non-Aura enchantments have no flash"
    );
}
