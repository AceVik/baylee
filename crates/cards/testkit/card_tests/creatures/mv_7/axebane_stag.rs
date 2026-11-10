//! `cards/creatures/mv_7/axebane_stag.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn axebane_stag() -> CardIndex {
    card_index("a45d7135-c042-40d9-818c-74fa1c7af8ad")
}

/// Axebane Stag — a vanilla 6/7 for {6}{G}: offered only once seven mana are
/// in the pool, and a 6/7 on the battlefield with no keyword after it
/// resolves; attacking unblocked it deals 6.
#[test]
fn axebane_stag_is_a_vanilla_six_seven_for_seven() {
    let mut engine = table7(&lots(forest(), 7), &[axebane_stag()], &[]);
    let stag = cast_and_resolve(&mut engine, axebane_stag());
    assert_body(&engine, stag, (6, 7), KeywordSet::default());
    assert_eq!(keywords(&engine, stag), KeywordSet::default(), "vanilla");
}

#[test]
fn axebane_stag_hits_for_six() {
    let (mut engine, stag, _ogre) = duel_vs_ogre(axebane_stag(), &[]);
    fight(&mut engine, &[stag], &[]);
    assert_eq!(life_of(&engine, 1), 14);
}
