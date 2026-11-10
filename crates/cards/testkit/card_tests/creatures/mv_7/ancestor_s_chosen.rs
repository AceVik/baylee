//! `cards/creatures/mv_7/ancestor_s_chosen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn ancestor_s_chosen_has_first_strike_and_gains_life_for_each_card_in_your_graveyard_on_etb() {
    let chosen = card_index("fc2ccab7-cab1-4463-b73d-898070136d74");
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(plains(), 7), &[chosen], &[]);
    seed_graveyard(&mut engine, p0, 5);
    let life_before = life_of(&engine, 0);

    let perm = cast_and_resolve(&mut engine, chosen);
    assert!(keywords(&engine, perm).contains(KeywordSet::FIRST_STRIKE));
    assert_eq!(life_of(&engine, 0), life_before + 5);
}
