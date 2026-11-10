//! `cards/creatures/artifacts/mv_4/su_chi.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Su-Chi` prints `When this creature dies, add {{C}}{{C}}{{C}}{{C}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 4/4 artifact creature with no floating mana in the pool.
/// When the opponent destroys it with `vindicate()`, the dies trigger is put on the stack,
/// resolves, and leaves exactly four colorless mana in seat 0's mana pool while `Su-Chi` rests in the graveyard.
#[test]
fn su_chi_adds_four_colorless_mana_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[su_chi()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    let chi = on_battlefield(&engine, p0, su_chi()).expect("su-chi is seated");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![chi] })
        .expect("vindicate targets su-chi");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, su_chi()).is_some(),
        "`Su-Chi` is in the graveyard"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        4,
        "the dies trigger produced four colorless mana"
    );
    assert_eq!(pool.total(), 4, "and nothing else floats in seat 0's pool");
}
