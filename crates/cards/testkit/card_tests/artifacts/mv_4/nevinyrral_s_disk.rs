//! `cards/artifacts/mv_4/nevinyrral_s_disk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nevinyrral's Disk: "This artifact enters tapped." / "{1}, {T}: Destroy
/// all artifacts, creatures, and enchantments." Cast (not seeded) to prove
/// the tapped entry; the sweep takes itself, both players' creatures and an
/// enchantment, and leaves every land standing.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end
fn nevinyrral_s_disk_enters_tapped_and_destroys_artifacts_creatures_and_enchantments() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                quiet_creature(),
                sterling_grove(),
            ],
        )
        .hand(0, &[nevinyrral_s_disk()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, nevinyrral_s_disk());
    pass_until(&mut engine, stack_is_empty);

    let disk = on_battlefield(&engine, p0, nevinyrral_s_disk()).expect("resolved");
    assert!(is_tapped(&engine, disk), "\"enters tapped\"");

    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(disk)
        .expect("seated")
        .status
        .remove(Status::TAPPED);
    engine.refresh_offer();
    activate(&mut engine, p0, nevinyrral_s_disk(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, nevinyrral_s_disk()).is_some(),
        "the Disk destroys itself too — it is an artifact"
    );
    assert!(on_battlefield(&engine, p0, quiet_creature()).is_none());
    assert!(on_battlefield(&engine, p1, quiet_creature()).is_none());
    assert!(on_battlefield(&engine, p0, sterling_grove()).is_none());
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        5,
        "lands are none of the three named types"
    );
}
