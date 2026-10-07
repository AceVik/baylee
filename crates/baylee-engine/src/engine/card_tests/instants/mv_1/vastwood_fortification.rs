//! `cards/instants/mv_1/vastwood_fortification.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vastwood Fortification, cast as its **front** face: one +1/+1 counter.
///
/// A modal double-faced card is two cards in one, and the half that is not
/// `play_land_face` is this one: the spell is cast out of the same hand the
/// land would have been played from, and the counter is what says which face
/// the engine took.
#[test]
fn vastwood_fortification_puts_a_counter_on_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(413, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves()])
        .hand(0, &[vastwood_fortification()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, vastwood_fortification());
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, elf, CounterKind::P1P1),
        1,
        "one +1/+1 counter, which is the whole of the front face"
    );
    assert_eq!(pt(&engine, elf), (2, 2), "so a 1/1 is a 2/2");
}
