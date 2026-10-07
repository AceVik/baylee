//! `cards/creatures/mv_2/lord_of_atlantis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lord of Atlantis — "Other Merfolk get +1/+1 and have islandwalk." The
/// lord does not pump itself, a non-Merfolk beside it is untouched, and the
/// other Merfolk gets both the bonus and the keyword.
#[test]
fn lord_of_atlantis_pumps_other_merfolk_and_grants_islandwalk() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                lord_of_atlantis(),
                merfolk_of_the_pearl_trident(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    let lord = on_battlefield(&engine, p0, lord_of_atlantis()).expect("seated");
    let merfolk =
        on_battlefield(&engine, p0, merfolk_of_the_pearl_trident()).expect("another Merfolk");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("not a Merfolk");

    assert_eq!(pt(&engine, lord), (2, 2), "it does not pump itself");
    assert!(!keywords(&engine, lord).contains(KeywordSet::ISLANDWALK));
    assert_eq!(
        pt(&engine, merfolk),
        (2, 2),
        "1/1 printed, +1/+1 from the lord"
    );
    assert!(keywords(&engine, merfolk).contains(KeywordSet::ISLANDWALK));
    assert_eq!(pt(&engine, elf), (1, 1), "not a Merfolk: untouched");
    assert!(!keywords(&engine, elf).contains(KeywordSet::ISLANDWALK));
}
