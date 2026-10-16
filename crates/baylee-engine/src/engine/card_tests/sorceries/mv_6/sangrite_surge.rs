//! `cards/sorceries/mv_6/sangrite_surge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "d8053319-b2a0-4df3-9787-3591d12cf1b9"
fn sangrite_surge() -> CardIndex {
    card_index("d8053319-b2a0-4df3-9787-3591d12cf1b9")
}

/// Sangrite Surge: "Target creature gets +3/+3 and gains double strike until
/// end of turn."
#[test]
fn sangrite_surge_pumps_and_grants_double_strike_until_end_of_turn() {
    let lands = [vec![mountain(); 5], vec![forest()]].concat();
    let mut engine = table(972, &[sangrite_surge()], &lands, &[], &[gray_ogre()]);
    let ogre = on_battlefield(&engine, them(), gray_ogre()).expect("their Ogre");
    assert_eq!(pt(&engine, ogre), (2, 2));
    assert!(!keywords_of(&engine, ogre).contains(KeywordSet::DOUBLE_STRIKE));
    cast_at(&mut engine, sangrite_surge(), ogre);
    assert_eq!(pt(&engine, ogre), (5, 5), "+3/+3");
    assert!(keywords_of(&engine, ogre).contains(KeywordSet::DOUBLE_STRIKE), "double strike");

    pass_until(&mut engine, |e| e.state().turn.active == them());
    assert_eq!(pt(&engine, ogre), (2, 2), "until end of turn");
    assert!(!keywords_of(&engine, ogre).contains(KeywordSet::DOUBLE_STRIKE));
}
