//! `cards/lands/unlucky/razortrap_gorge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// All ten unlucky lands, and the three life totals their sentence reads.
///
/// Both seats start at twenty, so the duel that every other land test uses
/// only ever shows the tapped branch here too. What turns the land on is
/// **a** player at thirteen or less — including its own controller, which is
/// the reading a test written only against the opponent would never separate
/// from "an opponent has 13 or less life", a different card.
#[test]
fn an_unlucky_land_reads_every_life_total_including_its_own() {
    for (i, (oracle, name)) in UNTAPPED_WHEN_SOMEONE_IS_LOW.iter().enumerate() {
        let card = card_index(oracle);
        let seed = 1000 + u64::try_from(i).expect("ten rows");
        assert!(
            arrives_tapped(|| Duel::new(seed, forest()), card),
            "{name} entered untapped with both seats at twenty"
        );
        assert!(
            !arrives_tapped(|| Duel::new(seed, forest()).life(1, 13), card),
            "{name} entered tapped with an opponent at exactly thirteen"
        );
        assert!(
            !arrives_tapped(|| Duel::new(seed, forest()).life(0, 12), card),
            "{name} ignored its own controller's life total"
        );
    }

    // Thirteen is the boundary the card prints, so fourteen is the other
    // side of it — without this the whole cycle would pass with a `<` for a
    // `<=` and be wrong on exactly one life total.
    let gorge = card_index("8f69bd3a-244e-42d8-bfac-5a426f4b54b4");
    assert!(
        arrives_tapped(|| Duel::new(1020, forest()).life(1, 14), gorge),
        "fourteen is not thirteen or less"
    );
}
