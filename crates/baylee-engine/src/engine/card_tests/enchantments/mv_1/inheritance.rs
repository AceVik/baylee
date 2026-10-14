//! `cards/enchantments/mv_1/inheritance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn inheritance() -> CardIndex {
    card_index("742cb752-dd20-4b1f-8d96-7e9a8f864e75")
}

/// "Whenever a creature dies, you may pay {3}. If you do, draw a card."
#[test]
fn inheritance_draws_when_i_pay_and_not_when_i_decline() {
    for pay in [true, false] {
        let mut e = table(7211, &[], &[inheritance()], &[index::GRIZZLY_BEARS]);
        let victim = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
        let hand = hand_size(&e, P0);
        crate::sba::destroy(e.dev_state_mut(P0).unwrap(), victim);
        let Pending::Priority { player, .. } = e.pending().clone() else {
            panic!("priority expected")
        };
        e.apply(player, PlayerAction::PassPriority).unwrap();
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        drive(&mut e, &[], &[], &[], pay);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(hand_size(&e, P0), hand + usize::from(pay), "pay = {pay}");
    }
}
