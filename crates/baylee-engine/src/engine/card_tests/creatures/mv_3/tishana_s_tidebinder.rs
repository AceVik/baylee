//! `cards/creatures/mv_3/tishana_s_tidebinder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "…for as long as this creature remains on the battlefield." Swords to
/// Plowshares takes the Tidebinder away and the Strix has its flying and its
/// deathtouch back.
///
/// The rider was written `Duration::UntilEndOfTurn`, which is what the card
/// file's header claimed too — and both were wrong in the same direction:
/// the suppression is not a turn's effect, it is the Tidebinder's, and it
/// outlives the turn exactly as long as the Tidebinder does.
#[test]
fn the_strix_takes_its_keywords_back_when_the_tidebinder_leaves() {
    let (mut engine, p0, p1, strix) = a_strix_the_tidebinder_answered();
    let tidebinder = on_battlefield(&engine, p1, tishanas_tidebinder()).expect("it stayed");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana_but(&mut engine, p0, None);
    let stp = in_hand(&engine, p0, swords_to_plowshares()).expect("the sword is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: stp })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&tidebinder),
        "the tidebinder was not a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![tidebinder],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, tishanas_tidebinder()).is_none() && stack_is_empty(e)
    });

    let keywords = keywords_of(&engine, strix);
    assert!(
        keywords.contains(KeywordSet::FLYING) && keywords.contains(KeywordSet::DEATHTOUCH),
        "the tidebinder is gone and the strix is still stripped: {keywords:?}"
    );
}
