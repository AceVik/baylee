//! `cards/creatures/mv_7/delraich.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Delraich — {6}{B} 6/6 Creature — Horror:
/// "Trample." (Partial: sacrifice-three-black-creatures alternative cost unimplemented).
#[test]
fn delraich_casts_as_six_six_with_trample() {
    let mut engine = table7(&lots(swamp(), 7), &[index::DELRAICH], &[]);
    let delraich = cast_and_resolve(&mut engine, index::DELRAICH);
    assert_body(&engine, delraich, (6, 6), KeywordSet::TRAMPLE);
}
