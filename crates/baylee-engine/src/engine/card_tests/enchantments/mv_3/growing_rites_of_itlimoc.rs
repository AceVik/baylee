//! `cards/enchantments/mv_3/growing_rites_of_itlimoc.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Growing Rites of Itlimoc` // `Itlimoc, Cradle of the Sun` (`Coverage::Partial`):
/// "When `Growing Rites of Itlimoc` enters, look at the top four cards of your library. You may
/// reveal a creature card from among them and put it into your hand. Put the rest on the bottom
/// of your library in any order. At the beginning of your end step, if you control four or more
/// creatures, transform `Growing Rites of Itlimoc`. // `{{T}}`: Add `{{G}}`. `{{T}}`: Add `{{G}}` for
/// each creature you control."
///
/// Under `Coverage::Partial`, the enter look-at-four trigger is omitted, while the end-step
/// transform trigger and the back face's mana abilities are implemented. The test sets up four
/// controlled creatures, advances to the end step where the transform condition is met, verifies
/// the enchantment transforms into the legendary land `Itlimoc, Cradle of the Sun` on face 1,
/// and activates Itlimoc's second mana ability to produce green mana equal to the creature count.
#[test]
fn growing_rites_of_itlimoc_transforms_at_four_creatures_and_taps_for_creature_count() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(106, forest())
        .battlefield(
            0,
            &[
                growing_rites_of_itlimoc(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let rites = on_battlefield(&engine, p0, growing_rites_of_itlimoc()).expect("the Rites");
    let rites_was = identity(&engine, rites);

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    pass_until(&mut engine, stack_is_empty);

    let itlimoc =
        on_battlefield(&engine, p0, growing_rites_of_itlimoc()).expect("Itlimoc on battlefield");
    assert_eq!(
        engine.state().object(itlimoc).map(|o| o.face_index),
        Some(1),
        "Growing Rites of Itlimoc transformed to face 1"
    );
    assert_eq!(
        identity(&engine, itlimoc),
        rites_was,
        "the same object, turned over: a transform changes no zone (CR 712.18)"
    );

    let t = types(&engine, itlimoc);
    assert!(t.contains(TypeSet::LAND), "Itlimoc is a land");
    assert!(
        !t.contains(TypeSet::ENCHANTMENT),
        "Itlimoc is not an enchantment"
    );

    activate(&mut engine, p0, growing_rites_of_itlimoc(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        4,
        "Itlimoc added four green mana for the four creatures controlled"
    );
}
