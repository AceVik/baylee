//! `cards/instants/mv_2/tear_asunder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tear Asunder — `{1}{G}` Instant with kicker `{1}{B}`.
/// "Exile target artifact or enchantment. If this spell was kicked,
/// exile target nonland permanent instead."
///
/// The card is a GENERATED STUB — no abilities are implemented. The stub
/// declares no targeting, so it casts as a vanilla spell. The test can
/// only confirm the card is registered and resolves into the graveyard.
/// The exile effect, the artifact-or-enchantment targeting filter, the
/// kicker mechanic, and the broadened "nonland permanent" targeting when
/// kicked are all absent; this scenario proves nothing about those clauses.
///
/// SKIP: the core effect — "exile target artifact or enchantment" — needs
/// a `ChooseTargets` step filtered to artifacts and enchantments. The
/// kicker variant needs the kicker declaration during casting and a
/// second, broader targeting step. Both depend on the exile ability and
/// Tear Asunder (`Coverage::Partial`): "Exile target artifact or
/// enchantment."
///
/// The ordinary cast keeps its artifact/enchantment filter. Kicked casts
/// and their replacement targets are exercised in `tear_asunder_tests`.
#[test]
fn tear_asunder_exiles_an_artifact_and_refuses_an_empty_board() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(303, forest())
        .battlefield(0, &[forest(), swamp()])
        .hand(0, &[tear_asunder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Nothing to exile: the target requirement is unsatisfiable and the
    // spell is refused rather than cast into nothing.
    let spell = in_hand(&engine, p0, tear_asunder()).expect("the spell is in hand");
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "with no artifact and no enchantment on the battlefield there is \
         nothing for it to target"
    );

    // The same spell on a board with a Mox Opal on it.
    let mut engine = Duel::new(303, forest())
        .battlefield(0, &[forest(), swamp(), mox_opal()])
        .hand(0, &[tear_asunder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mox = on_battlefield(&engine, p0, mox_opal()).expect("the Mox is on the battlefield");
    cast_from_hand(&mut engine, p0, tear_asunder());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Tear Asunder asks for an artifact or enchantment, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&mox),
        "an artifact is what the filter admits: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![mox] })
        .expect("the Mox is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, mox_opal()).is_none(),
        "an exiled permanent leaves the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, mox_opal()).is_none(),
        "exile is not the graveyard (CR 406.1)"
    );
}
