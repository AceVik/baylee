//! `cards/creatures/mv_3/katara_the_fearless.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The owner's table (report 01a0e3d2, #318): Sokka, Tenacious Tactician
/// beside Katara, the Fearless, and a noncreature spell cast.
///
/// Sokka: "Menace, prowess … Other Allies you control have menace and
/// prowess. Whenever you cast a noncreature spell, create a 1/1 white Ally
/// creature token." Katara: "If a triggered ability of an Ally you control
/// triggers, that ability triggers an additional time." Prowess is a
/// triggered ability (CR 702.108a), so each Ally's prowess triggers twice
/// (CR 603.2d) and each gets +2/+2, as the token ability triggers twice.
/// The engine made the two tokens and grew each Ally by +1/+1 only: Katara
/// never reached a keyword's trigger.
#[test]
fn katara_doubles_the_prowess_sokka_prints_and_the_prowess_he_lends() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sokka_tenacious_tactician(), katara_the_fearless()])
        .hand(0, &[mox_opal()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let sokka = on_battlefield(&engine, p0, sokka_tenacious_tactician()).expect("Sokka is out");
    let katara = on_battlefield(&engine, p0, katara_the_fearless()).expect("Katara is out");
    assert_eq!(pt(&engine, sokka), (3, 3));
    assert_eq!(pt(&engine, katara), (3, 3));
    assert!(tokens_of(&engine, p0).is_empty());

    cast_from_hand(&mut engine, p0, mox_opal());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        tokens_of(&engine, p0).len(),
        2,
        "the token ability triggered twice, as it already did"
    );
    assert_eq!(
        pt(&engine, sokka),
        (5, 5),
        "Sokka's own prowess triggered twice: +2/+2, not +1/+1"
    );
    assert_eq!(
        pt(&engine, katara),
        (5, 5),
        "and so did the prowess Sokka gives Katara"
    );
}
