//! Oracle: draw two cards.
#[allow(clippy::wildcard_imports)]
use super::*;
fn quick_study() -> CardIndex {
    card_index("7bff8d4a-1c7d-48b8-b3e3-737dd6f01823")
}
/// Checklist: no target or choice; exactly two cards drawn by caster.
#[test]
fn draws_two_cards() {
    let p = PlayerId::new(0);
    let mut g = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[quick_study()])
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    let l = library_size(&g, p);
    let h = g.state().zones.list(ZoneLocation::Hand(p)).len();
    cast_from_hand(&mut g, p, quick_study());
    pass_until(&mut g, stack_is_empty);
    assert_eq!(library_size(&g, p), l - 2);
    assert_eq!(g.state().zones.list(ZoneLocation::Hand(p)).len(), h + 1);
}
