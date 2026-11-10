//! `cards/creatures/artifacts/mv_3/cathodion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Cathodion` prints `When this creature dies, add {{C}}{{C}}{{C}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 3/3 artifact creature while holding no floating mana.
/// When the opponent destroys it with `vindicate()`, the dies trigger goes onto the stack,
/// resolves, and leaves exactly three colorless mana in seat 0's mana pool while the creature lies in the graveyard.
#[test]
fn cathodion_adds_three_colorless_mana_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[cathodion()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    let cat = on_battlefield(&engine, p0, cathodion()).expect("cathodion is seated");
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![cat] })
        .expect("vindicate targets cathodion");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, cathodion()).is_some(),
        "`Cathodion` should be in the graveyard"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        3,
        "the dies trigger produced three colorless mana"
    );
    assert_eq!(pool.total(), 3, "and nothing else floats in seat 0's pool");
}
