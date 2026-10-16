//! `cards/creatures/mv_7/whiptail_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn whiptail_wurm() -> CardIndex {
    card_index("50fa6a63-e031-47cb-8fd5-a6c235203722")
}

/// Whiptail Wurm — a vanilla 8/5 for {6}{G}.
#[test]
fn whiptail_wurm_is_a_vanilla_eight_five_for_seven() {
    let mut engine = table7(&lots(forest(), 7), &[whiptail_wurm()], &[]);
    let wurm = cast_and_resolve(&mut engine, whiptail_wurm());
    assert_body(&engine, wurm, (8, 5), KeywordSet::default());
    assert_eq!(keywords(&engine, wurm), KeywordSet::default(), "vanilla");
}

#[test]
fn whiptail_wurm_hits_for_eight() {
    let (mut engine, wurm, _ogre) = duel_vs_ogre(whiptail_wurm(), &[]);
    fight(&mut engine, &[wurm], &[]);
    assert_eq!(life_of(&engine, 1), 12);
}
