//! `cards/creatures/mv_5/general_tazri.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// General Tazri: "When General Tazri enters, you may search your library
/// for an Ally creature card, reveal it, put it into your hand, then
/// shuffle."
#[test]
fn general_tazri_searches_for_an_ally_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4603, kazandu_blademaster())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[general_tazri()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let lib_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, general_tazri());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the search asks")
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, kazandu_blademaster()).is_some(),
        "the Ally is in hand"
    );
    assert_eq!(library_size(&engine, p0), lib_before - 1);
}
