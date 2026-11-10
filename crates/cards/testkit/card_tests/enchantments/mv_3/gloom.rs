//! `cards/enchantments/mv_3/gloom.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gloom pays its printed cost and resolves; its two taxes are tested in
/// the `gloom` submodule.
#[test]
fn alpha_eval_gloom_supported_cast_and_resolution() {
    let p0 = PlayerId::new(0);
    let card = card_index("4d022f53-b1fb-4071-afcc-0af3214fe604");
    let mut engine = Duel::new(1001, forest())
        .battlefield(0, &[swamp(); 3])
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
