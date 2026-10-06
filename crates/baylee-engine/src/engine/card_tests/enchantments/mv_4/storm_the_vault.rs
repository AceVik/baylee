//! `cards/enchantments/mv_4/storm_the_vault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Storm the Vault` // `Vault of Catlacan` (`Coverage::Partial`):
/// "Whenever one or more creatures you control deal combat damage to a player, create a Treasure
/// token. At the beginning of your end step, if you control five or more artifacts, transform
/// `Storm the Vault`. // `{{T}}`: Add one mana of any color. `{{T}}`: Add `{{U}}` for each artifact
/// you control."
///
/// Controlling five artifacts satisfies the end-step transform condition. The test sets up five
/// `quiet_artifact()`s, advances to the end step where the transform trigger resolves, verifies
/// `Storm the Vault` becomes the legendary land `Vault of Catlacan` on face 1, and activates its
/// second mana ability to produce blue mana equal to the artifact count. It is the same
/// permanent, turned over (CR 712.18), and not a new object that entered.
#[test]
fn storm_the_vault_transforms_at_five_artifacts_and_taps_for_artifact_count() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(110, island())
        .battlefield(
            0,
            &[
                storm_the_vault(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let storm = on_battlefield(&engine, p0, storm_the_vault()).expect("Storm the Vault");
    let storm_was = identity(&engine, storm);

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    pass_until(&mut engine, stack_is_empty);

    let vault =
        on_battlefield(&engine, p0, storm_the_vault()).expect("Vault of Catlacan on battlefield");
    assert_eq!(
        engine.state().object(vault).map(|o| o.face_index),
        Some(1),
        "Storm the Vault transformed to face 1"
    );
    assert_eq!(
        identity(&engine, vault),
        storm_was,
        "the same object, turned over: a transform changes no zone (CR 712.18)"
    );

    let t = types(&engine, vault);
    assert!(t.contains(TypeSet::LAND), "Vault of Catlacan is a land");
    assert!(
        !t.contains(TypeSet::ENCHANTMENT),
        "Vault of Catlacan is not an enchantment"
    );

    activate(&mut engine, p0, storm_the_vault(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        5,
        "Vault of Catlacan produced five blue mana for the five artifacts controlled"
    );
}
