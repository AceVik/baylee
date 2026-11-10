//! Oracle: when this enters, you draw a card and lose 1 life.
#[allow(clippy::wildcard_imports)]
use super::*;
fn dusk_legion_zealot() -> CardIndex {
    card_index("ea2dce18-195e-4681-a687-f9819edaf9fc")
}
/// Checklist: actual cast causes one draw and one life loss for controller.
#[test]
fn entering_draws_one_and_costs_one_life() {
    let p = PlayerId::new(0);
    let mut g = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[dusk_legion_zealot()])
        .life(0, 9)
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    let l = library_size(&g, p);
    let h = g.state().zones.list(ZoneLocation::Hand(p)).len();
    cast_from_hand(&mut g, p, dusk_legion_zealot());
    pass_until(&mut g, stack_is_empty);
    assert_eq!(library_size(&g, p), l - 1);
    assert_eq!(g.state().players[0].life, 8);
    assert_eq!(g.state().zones.list(ZoneLocation::Hand(p)).len(), h);
    assert!(on_battlefield(&g, p, dusk_legion_zealot()).is_some());
}
