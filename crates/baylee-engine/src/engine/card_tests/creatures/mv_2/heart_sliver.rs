//! `cards/creatures/mv_2/heart_sliver.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Heart Sliver` prints a static ability granting `KeywordSet::HASTE` to all Sliver creatures
/// under `Coverage::Implemented`.
/// On a board with `Heart Sliver` and a non-Sliver creature (`llanowar_elves`), the Sliver possesses haste
/// while the Elf does not, verifying that the keyword grant applies selectively to Slivers.
#[test]
fn heart_sliver_grants_haste_to_slivers() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1629, forest())
        .battlefield(0, &[heart_sliver(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sliver = on_battlefield(&engine, p0, heart_sliver()).expect("sliver is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf is seated");

    assert!(keywords(&engine, sliver).contains(KeywordSet::HASTE));
    assert!(!keywords(&engine, elf).contains(KeywordSet::HASTE));
}
