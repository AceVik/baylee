//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn planar_gate_reductions_stack_and_apply_to_both_x_symbols_once() {
    let p0 = PlayerId::new(0);
    for (copies, mana, x) in [(1, 0, 1), (1, 2, 2), (2, 0, 2), (2, 2, 3)] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &vec![index::PLANAR_GATE; copies])
            .hand(0, &[index::WALKING_BALLISTA])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, mana);
        e.refresh_offer();
        cast_with_floating(&mut e, p0, index::WALKING_BALLISTA);
        assert!(matches!(e.pending(),Pending::ChooseNumber{min:0,max,..} if *max>=x));
        e.apply(p0, PlayerAction::ChooseNumber(x)).unwrap();
        assert_eq!(e.state().players[0].mana_pool.total(), 0);
        pass_until(&mut e, stack_is_empty);
        let ballista = on_battlefield(&e, p0, index::WALKING_BALLISTA).unwrap();
        assert_eq!(u32::from(counters_on(&e, ballista, CounterKind::P1P1)), x);
    }
}
#[test]
fn planar_gate_reduces_after_gloom_tax_without_erasing_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::PLANAR_GATE, index::GLOOM])
        .hand(0, &[index::SAVANNAH_LIONS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 1);
    e.refresh_offer();
    let card = in_hand(&e, p0, index::SAVANNAH_LIONS).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.castable.contains(&card));
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    e.refresh_offer();
    cast_with_floating(&mut e, p0, index::SAVANNAH_LIONS);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, index::SAVANNAH_LIONS).is_some());
}
