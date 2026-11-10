//! `cards/lands/utility/riptide_laboratory.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Riptide Laboratory returns a **Wizard you control**, and both words in
/// that phrase are a refusal.
///
/// Three creatures stand on the board and only one of them is a legal
/// target: a Wizard of mine, a Wizard the opponent controls, and a
/// nonWizard of mine. A filter that had lost `ControlledByYou` would offer
/// the first two, a filter that had lost the subtype would offer the first
/// and the third, and either mistake is invisible on a board with one
/// creature on it — which is how a filter with one half missing ships.
#[test]
fn riptide_laboratory_returns_a_wizard_you_control_and_no_other_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4_840, forest())
        .battlefield(
            0,
            &[
                riptide_laboratory(),
                island(),
                island(),
                ana_disciple(),
                ignoble_hierarch(),
            ],
        )
        .battlefield(1, &[ana_disciple()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lab = on_battlefield(&engine, p0, riptide_laboratory()).expect("the Lab is on the table");
    let mine = on_battlefield(&engine, p0, ana_disciple()).expect("my Wizard is on the table");
    let theirs = on_battlefield(&engine, p1, ana_disciple()).expect("and so is theirs");
    let other = on_battlefield(&engine, p0, ignoble_hierarch()).expect("and my nonWizard creature");
    tap_mana_except(&mut engine, p0, lab);

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: lab,
                ability_index: 1,
            },
        )
        .expect("{1}{U} is floating and the Lab is untapped");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![mine],
        "\"target Wizard you control\" is one creature on this board, and \
         the other two are the two ways the filter could have been written \
         wrong: {theirs:?} is a Wizard I do not control, {other:?} is a \
         creature I control that is no Wizard"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the one creature the menu named is a legal answer");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_hand(&engine, p0, ana_disciple()).is_some(),
        "my Wizard went to my hand"
    );
    assert!(
        on_battlefield(&engine, p1, ana_disciple()).is_some(),
        "and theirs never moved"
    );
}
