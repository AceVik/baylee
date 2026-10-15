//! Oracle: when this enters, you gain 3 life.
#[allow(clippy::wildcard_imports)]
use super::*;
fn healer_of_the_glade() -> CardIndex {
    card_index("2701b912-b672-4764-97f7-51c444f754f2")
}
/// Checklist: cast entry trigger gains exactly three life for its controller.
#[test]
fn entry_trigger_gains_three_life() {
    let p = PlayerId::new(0);
    let mut g = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[healer_of_the_glade()])
        .life(0, 14)
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    cast_from_hand(&mut g, p, healer_of_the_glade());
    pass_until(&mut g, stack_is_empty);
    assert_eq!(g.state().players[0].life, 17);
    assert!(on_battlefield(&g, p, healer_of_the_glade()).is_some());
}
