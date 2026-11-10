//! `cards/creatures/mv_1/ana_disciple.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ana Disciple` prints `{{U}}, {{T}}: Target creature gains flying until end of turn.` and `{{B}}, {{T}}: Target creature gets -2/-0 until end of turn.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Ana Disciple`, an Island, and a `llanowar_elves()`.
/// Floating blue mana and activating the first ability targets the elf, taps `Ana Disciple`, and grants the elf `KeywordSet::FLYING` until end of turn.
#[test]
fn ana_disciple_taps_with_blue_mana_to_grant_flying() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ana_disciple(), island(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let disciple = on_battlefield(&engine, p0, ana_disciple()).expect("disciple seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf seated");
    assert!(!is_tapped(&engine, disciple));
    assert!(!keywords(&engine, elf).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );

    activate(&mut engine, p0, ana_disciple(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf), "elf is a legal target creature");

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("target elf chosen");

    assert!(
        is_tapped(&engine, disciple),
        "`Ana Disciple` tapped to pay activation cost"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, elf).contains(KeywordSet::FLYING),
        "target elf gained flying until end of turn"
    );
}
