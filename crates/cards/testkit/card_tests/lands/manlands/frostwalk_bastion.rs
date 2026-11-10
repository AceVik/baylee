//! `cards/lands/manlands/frostwalk_bastion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Frostwalk Bastion: "{T}: Add {C}." / "{1}{S}: Until end of turn, this land becomes a 2/3 Construct artifact creature. It's still a land."
/// Under `Coverage::Partial`, the combat-damage trigger is omitted because `Trigger` has no variant for damage to a creature.
/// Paying `{1}{S}` animates the land into a 2/3 Construct artifact creature while retaining its land type.
#[test]
fn frostwalk_bastion_animates_into_artifact_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(214, forest())
        .battlefield(0, &[frostwalk_bastion(), mouth_of_ronom(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bastion = on_battlefield(&engine, p0, frostwalk_bastion()).expect("Bastion deployed");
    // `{1}{S}` wants a colorless mana and the Forest cannot make one; the
    // Bastion could, but its animation cost has no `{T}`, so paying with
    // itself would leave it tapped and the last assertion below is that it
    // is not. The Mouth is the snow source and `tap_mana_except` now taps it
    // with the rest (#159) — it stays the right board once `{S}` means
    // "from a snow source" (#158).
    let mouth = on_battlefield(&engine, p0, mouth_of_ronom()).expect("the Mouth stands");
    tap_mana_except(&mut engine, p0, bastion);
    assert!(
        is_tapped(&engine, mouth),
        "the Mouth made the colorless mana"
    );
    activate(&mut engine, p0, frostwalk_bastion(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bastion), (2, 3));
    let types = engine
        .state()
        .object(bastion)
        .unwrap()
        .characteristics()
        .types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::ARTIFACT));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, bastion));
}
