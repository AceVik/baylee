//! `cards/creatures/enchantments/mv_3/courser_of_kruphix.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Courser of Kruphix` prints `Play with the top card of your library revealed.`, `You may play lands from the top of your library.`, and `Landfall — Whenever a land you control enters, you gain 1 life.`
///
/// Landfall applies to a land played from hand too, and the ordinary land
/// limit still applies when Courser opens the top of the library.
#[test]
fn courser_of_kruphix_gains_life_on_land_entry_and_respects_the_land_limit() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[courser_of_kruphix()])
        .hand(0, &[forest()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let courser =
        on_battlefield(&engine, p0, courser_of_kruphix()).expect("courser on battlefield");
    assert_eq!(pt(&engine, courser), (2, 4));
    assert_eq!(engine.state().players[0].life, 20);

    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 21, "landfall gained 1 life");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.lands.is_empty(),
        "the normal land drop was already used"
    );
}
