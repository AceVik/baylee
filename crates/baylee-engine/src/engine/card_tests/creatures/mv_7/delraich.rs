//! `cards/creatures/mv_7/delraich.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn subject_card() -> CardIndex {
    card_index("38f9e766-a2ad-437b-a38c-d6f3ffd261fb")
}

/// Delraich — {6}{B} 6/6 Creature — Horror:
/// "Trample." (Partial: sacrifice-three-black-creatures alternative cost unimplemented).
#[test]
fn delraich_casts_as_six_six_with_trample() {
    let mut engine = table7(&lots(swamp(), 7), &[subject_card()], &[]);
    let delraich = cast_and_resolve(&mut engine, subject_card());
    assert_body(&engine, delraich, (6, 6), KeywordSet::TRAMPLE);
}
