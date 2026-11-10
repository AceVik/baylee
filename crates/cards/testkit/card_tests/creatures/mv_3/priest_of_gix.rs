//! `cards/creatures/mv_3/priest_of_gix.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Priest of Gix prints one line and it is the whole card: `{2}{B}` for a 2/1
/// whose enters-trigger adds `{B}{B}{B}`. That mana is the point, so the board
/// is three Swamps and nothing else — the pool is read **empty** while the
/// creature is still on the stack, which is what says the `{2}{B}` was really
/// spent, and then exactly three black appear off the trigger, which no
/// permanent left standing could have produced. The 2/1 body is read beside it
/// so that a trigger resolving against nothing would not pass on the pool
/// alone.
#[test]
fn priest_of_gix_spends_three_swamps_and_hands_back_three_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[priest_of_gix()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, priest_of_gix());
    // Cost is paid before the creature is anywhere, so an empty pool here is
    // what makes the three black below the trigger's and not three the cast
    // never spent.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the three Swamps paid the {{2}}{{B}} and there were exactly three of them"
    );
    assert!(
        on_stack(&engine, priest_of_gix()).is_some(),
        "the creature is on the stack, not yet on the battlefield"
    );

    pass_until(&mut engine, stack_is_empty);

    let priest = on_battlefield(&engine, p0, priest_of_gix()).expect("the Priest resolved");
    assert_eq!(pt(&engine, priest), (2, 1), "the printed 2/1 body");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        3,
        "\"When this creature enters, add {{B}}{{B}}{{B}}\""
    );
    assert_eq!(
        pool.total(),
        3,
        "three black and nothing else: every Swamp on the board is tapped and \
         no permanent left there makes mana at all"
    );
}
