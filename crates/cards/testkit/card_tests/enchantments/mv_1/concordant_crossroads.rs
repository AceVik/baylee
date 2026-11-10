//! `cards/enchantments/mv_1/concordant_crossroads.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Concordant Crossroads` (`Coverage::Implemented`):
/// "All creatures have haste."
///
/// Verifies that while `Concordant Crossroads` is on the battlefield, all creatures
/// on both sides of the table gain `KeywordSet::HASTE`.
#[test]
fn concordant_crossroads_grants_haste_to_all_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1318, forest())
        .battlefield(0, &[forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[concordant_crossroads()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my elf deployed");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their elf deployed");
    assert!(!keywords(&engine, mine).contains(KeywordSet::HASTE));
    assert!(!keywords(&engine, theirs).contains(KeywordSet::HASTE));

    cast_from_hand(&mut engine, p0, concordant_crossroads());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, concordant_crossroads()).is_some());
    assert!(
        keywords(&engine, mine).contains(KeywordSet::HASTE),
        "my creature gains haste"
    );
    assert!(
        keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "opponent creature also gains haste"
    );
}
