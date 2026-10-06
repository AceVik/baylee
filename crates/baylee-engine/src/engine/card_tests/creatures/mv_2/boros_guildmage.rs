//! `cards/creatures/mv_2/boros_guildmage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Boros Guildmage` prints two activated abilities under `Coverage::Implemented`:
/// `{{1}}{{R}}` granting haste until end of turn, and `{{1}}{{W}}` granting first strike until end of turn.
/// Activating both abilities in sequence targeting the Guildmage verifies that
/// `KeywordSet::HASTE` and `KeywordSet::FIRST_STRIKE` are successfully granted via the layer system.
#[test]
fn boros_guildmage_grants_haste_and_first_strike() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1601, forest())
        .battlefield(
            0,
            &[
                boros_guildmage(),
                mountain(),
                mountain(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let guildmage =
        on_battlefield(&engine, p0, boros_guildmage()).expect("Guildmage is on battlefield");
    assert!(!keywords(&engine, guildmage).contains(KeywordSet::HASTE));
    assert!(!keywords(&engine, guildmage).contains(KeywordSet::FIRST_STRIKE));

    tap_all_mana(&mut engine, p0);

    activate(&mut engine, p0, boros_guildmage(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Boros Guildmage haste ability");
    };
    assert!(options.contains(&guildmage));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![guildmage],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, guildmage).contains(KeywordSet::HASTE));
    assert!(!keywords(&engine, guildmage).contains(KeywordSet::FIRST_STRIKE));

    activate(&mut engine, p0, boros_guildmage(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Boros Guildmage first strike ability");
    };
    assert!(options.contains(&guildmage));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![guildmage],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let kw = keywords(&engine, guildmage);
    assert!(kw.contains(KeywordSet::HASTE));
    assert!(kw.contains(KeywordSet::FIRST_STRIKE));
}
