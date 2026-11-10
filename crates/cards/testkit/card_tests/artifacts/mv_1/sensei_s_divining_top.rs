//! `cards/artifacts/mv_1/sensei_s_divining_top.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Sensei's Divining Top` is an artifact costing `{1}` under `Coverage::Implemented`.
/// It prints "{T}: Draw a card, then put this artifact on top of its owner's library."
/// When its second ability is activated, it taps, resolves to draw one card,
/// leaves the battlefield, and places itself on top of its owner's library.
#[test]
fn senseis_divining_top_draws_and_replaces_itself_on_top_of_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[sensei_s_divining_top()])
        .hand(0, &[])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let top_obj = on_battlefield(&engine, p0, sensei_s_divining_top())
        .expect("Sensei's Divining Top is on the battlefield");

    activate(&mut engine, p0, sensei_s_divining_top(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, sensei_s_divining_top()).is_none(),
        "no longer on the battlefield"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "drew one card into hand"
    );

    let lib = engine.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(
        lib.last().copied(),
        Some(top_obj),
        "placed on top of its owner's library"
    );
}
