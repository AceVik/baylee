//! `cards/enchantments/mv_1/mirri_s_guile.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mirri's Guile: the upkeep question, and the library it leaves alone.
///
/// "You may look at the top three cards of your library, then put them back
/// in any order" moves no card between zones, so the only thing a game can
/// observe is that the question is asked and that answering it changes no
/// count — which is exactly what `Effect::ReorderTopLibrary` promises.
#[test]
fn mirri_s_guile_asks_at_upkeep_and_moves_no_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(399, forest())
        .battlefield(0, &[mirri_s_guile()])
        .start();
    keep_mulligans(&mut engine);

    let before = library_size(&engine, p0);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the reorder is answered on the way"
    );

    assert_eq!(
        library_size(&engine, p0),
        before,
        "three cards were looked at and put back, so the library is the size \
         it was"
    );
}
