//! `cards/enchantments/mv_2/sterling_grove.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sterling Grove: the tutor puts the card **on top**, and the enchantments
/// beside it are untargetable.
///
/// Both halves are on one board because the second is what the first costs:
/// the Grove sacrifices itself, so the shroud it was granting goes with it.
#[test]
fn sterling_grove_shrouds_its_neighbours_and_tutors_to_the_top() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(402, forest())
        // Fastbond and not an Aura: an Aura seated with no host is put into
        // the graveyard by state-based actions (CR 704.5m) before anything
        // can be asked about it.
        .battlefield(0, &[sterling_grove(), fastbond(), forest()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let neighbour =
        on_battlefield(&engine, p0, fastbond()).expect("the other enchantment is there");
    assert!(
        keywords(&engine, neighbour).contains(KeywordSet::SHROUD),
        "\"Other enchantments you control have shroud\""
    );
    let grove = on_battlefield(&engine, p0, sterling_grove()).expect("the Grove is there");
    assert!(
        !keywords(&engine, grove).contains(KeywordSet::SHROUD),
        "and \"other\" leaves the Grove itself out"
    );

    let before = library_size(&engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, sterling_grove(), 1);
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the search is answered on the way"
    );
    assert_eq!(
        library_size(&engine, p0),
        before,
        "the tutored card goes on top of the library rather than out of it"
    );
    assert!(
        on_battlefield(&engine, p0, sterling_grove()).is_none(),
        "and the Grove sacrificed itself to do it"
    );
    let _ = p1;
}
