//! `cards/enchantments/mv_4/manabarbs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Manabarbs damages whoever tapped the land, once per tap, after its
/// ordinary trigger resolves. A nonland mana source does not trigger it.
#[test]
fn alpha_eval_manabarbs_counts_land_taps_for_either_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let barbs = card_index("0f1afedd-c60f-454f-b84a-c8117aec0128");
    let mut engine = Duel::new(1004, forest())
        .battlefield(0, &[barbs, mountain(), sol_ring()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    activate(&mut engine, p0, sol_ring(), 0);
    assert!(stack_is_empty(&engine), "a rock is not a land");
    assert_eq!(engine.state().players[0].life, 20);
    for seat in [p0, p1] {
        pass_until(&mut engine, |e| at_rest(e, seat));
        let land = on_battlefield(&engine, seat, mountain()).unwrap();
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source: land })
            .unwrap();
        assert!(!stack_is_empty(&engine), "damage waits on the stack");
        assert_eq!(engine.state().players[usize::from(seat.get())].life, 20);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().players[usize::from(seat.get())].life, 19);
    }
    assert_eq!(engine.state().players[0].life, 19);
}
