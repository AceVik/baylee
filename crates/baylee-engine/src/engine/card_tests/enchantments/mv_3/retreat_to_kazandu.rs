//! `cards/enchantments/mv_3/retreat_to_kazandu.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Retreat to Kazandu: a landfall trigger with two modes, and the mode is
/// chosen every time it triggers.
///
/// `modal_triggered!` is the shape, and the half worth playing is that both
/// modes are reachable off the same land drop: the second land takes the
/// other one, so the counter and the two life are the same ability answering
/// two different questions.
#[test]
fn retreat_to_kazandu_offers_both_of_its_modes_on_a_land_drop() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(408, forest())
        .battlefield(0, &[retreat_to_kazandu(), llanowar_elves()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    let card = in_hand(&engine, p0, forest()).expect("the land is in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected the mode choice, got {:?}", engine.pending())
    };
    assert_eq!(options.len(), 2, "\"choose one\" of two");
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "the first mode put a +1/+1 counter on the target"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the second mode's two life was not also taken"
    );
}
