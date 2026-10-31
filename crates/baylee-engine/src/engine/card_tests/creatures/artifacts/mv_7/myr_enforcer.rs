//! Myr Enforcer's affinity is paid at the casting offer.

use super::*;

fn myr_enforcer() -> CardIndex {
    card_index("2d8e1054-654f-42e8-8c29-12c3cf13a3eb")
}

#[test]
fn myr_enforcer_costs_zero_with_seven_artifacts() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_artifact(); 7])
        .hand(0, &[myr_enforcer()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let spell = in_hand(&e, p0, myr_enforcer()).unwrap();
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected priority")
    };
    assert!(legal.castable.contains(&spell));
    e.apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, myr_enforcer()).is_some());
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}
