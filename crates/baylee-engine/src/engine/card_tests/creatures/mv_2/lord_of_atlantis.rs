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

/// "Other Merfolk" says nothing about who controls them. A Lord of Atlantis
/// we control pumps the opponent's Merfolk and gives it islandwalk, leaves
/// their Elf alone, and — two Lords on the table — each is "another Merfolk"
/// to the other, so each is 3/3 with islandwalk while neither pumps itself.
#[test]
fn lord_of_atlantis_reaches_the_opponents_merfolk_and_their_lord() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[lord_of_atlantis()])
        .battlefield(1, &[merfolk_of_the_pearl_trident(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let their_merfolk =
        on_battlefield(&engine, p1, merfolk_of_the_pearl_trident()).expect("their Merfolk");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");
    assert_eq!(
        pt(&engine, their_merfolk),
        (2, 2),
        "the opponent's Merfolk is 1/1 printed, +1/+1 from our lord"
    );
    assert!(keywords(&engine, their_merfolk).contains(KeywordSet::ISLANDWALK));
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "their non-Merfolk: untouched"
    );
    assert!(!keywords(&engine, their_elf).contains(KeywordSet::ISLANDWALK));

    let mut both = Duel::new(SEED, island())
        .battlefield(0, &[lord_of_atlantis()])
        .battlefield(1, &[lord_of_atlantis()])
        .start();
    keep_mulligans(&mut both);
    for seat in [p0, p1] {
        let lord = on_battlefield(&both, seat, lord_of_atlantis()).expect("a lord");
        assert_eq!(
            pt(&both, lord),
            (3, 3),
            "the other lord is another Merfolk: 2/2 printed, +1/+1"
        );
        assert!(keywords(&both, lord).contains(KeywordSet::ISLANDWALK));
    }
}
