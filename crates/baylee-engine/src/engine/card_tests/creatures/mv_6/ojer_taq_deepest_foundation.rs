//! `cards/creatures/mv_6/ojer_taq_deepest_foundation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ojer Taq, Deepest Foundation` // `Temple of Civilization` (`Coverage::Partial`):
/// "Vigilance. If one or more creature tokens would be created under your control, three times
/// that many of those tokens are created instead. When `Ojer Taq` dies, return it to the battlefield
/// tapped and transformed under its owner's control. // `{{T}}`: Add `{{W}}`. `{{2}}{{W}}`, `{{T}}`: Transform
/// this land. Activate only if you attacked with three or more creatures this turn and only as a sorcery."
///
/// Under `Coverage::Partial`, the token tripler, dies-transform, and transform-back are omitted,
/// leaving `KeywordSet::VIGILANCE` on a 6/6 God. The test verifies that `Ojer Taq` has 6/6 stats and vigilance,
/// attacks an opponent to deal 6 combat damage, and remains untapped afterwards due to vigilance.
#[test]
fn ojer_taq_deepest_foundation_attacks_with_vigilance_and_remains_untapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(311, plains())
        .battlefield(0, &[ojer_taq_deepest_foundation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let god = on_battlefield(&engine, p0, ojer_taq_deepest_foundation())
        .expect("Ojer Taq on battlefield");
    assert_eq!(pt(&engine, god), (6, 6), "Ojer Taq is a 6/6 God");
    assert!(
        keywords(&engine, god).contains(KeywordSet::VIGILANCE),
        "Ojer Taq has vigilance"
    );
    assert!(!is_tapped(&engine, god), "starts untapped");

    // Advance to combat and declare Ojer Taq as an attacker against p1.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(god, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Not `stack_is_empty`: the stack is already empty the moment attackers
    // are declared, so that predicate stops the walk *before* the combat
    // damage step and every life total still reads 20. The end step is past
    // damage (CR 510.2) and is what the assertion below needs.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        14,
        "opponent took 6 combat damage"
    );
    assert!(
        !is_tapped(&engine, god),
        "vigilance keeps Ojer Taq untapped after attacking"
    );
}
