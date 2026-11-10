//! `cards/artifacts/mv_4/cyclopean_tomb.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cyclopean Tomb is Partial: the activation and graveyard trigger is unsupported.
/// Exercise only the printed cost and normal spell resolution (CR 601.2h,
/// 608.3a); no assertion treats the missing text as a working ability.
#[test]
fn alpha_eval_cyclopean_tomb_supported_cast_and_resolution() {
    let p0 = PlayerId::new(0);
    let card = card_index("1edee40f-d153-4d67-aee7-ef08e11e4a79");
    let mut engine = Duel::new(1001, forest())
        .battlefield(0, &[forest(); 4])
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, card);
    assert!(on_stack(&engine, card).is_some(), "the card was cast");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the printed cost was paid"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, card).is_some());
    assert!(in_hand(&engine, p0, card).is_none());
}
