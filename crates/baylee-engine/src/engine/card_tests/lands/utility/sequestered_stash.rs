//! `cards/lands/utility/sequestered_stash.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sequestered Stash: "{T}: Add {C}." / "{4}, {T}, Sacrifice this land: Mill five cards. Then you may put an artifact card from your graveyard on top of your library."
/// Under `Coverage::Partial`, placing an artifact from graveyard on top of the library is unsupported.
/// Activating ability 1 for `{4}` and sacrificing this land mills five cards from the library into the graveyard.
#[test]
fn sequestered_stash_sacrifices_to_mill_five_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(135, forest())
        .battlefield(
            0,
            &[sequestered_stash(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stash =
        on_battlefield(&engine, p0, sequestered_stash()).expect("Sequestered Stash deployed");
    let lib_before = library_size(&engine, p0);

    tap_mana_except(&mut engine, p0, stash);
    activate(&mut engine, p0, sequestered_stash(), 1);

    assert!(in_graveyard(&engine, p0, sequestered_stash()).is_some());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), lib_before - 5);
}
