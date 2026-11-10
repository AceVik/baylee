//! `cards/creatures/mv_7/flora_colossus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn flora_colossus() -> CardIndex {
    card_index("04dce348-be42-443a-a88d-c47436ba2ec4")
}

/// Flora Colossus — hexproof, and "power and toughness are each equal to the
/// number of lands you control". Cast off seven Forests it is a 7/7.
#[test]
fn flora_colossus_is_as_big_as_the_lands_it_controls() {
    let mut engine = table7(&lots(forest(), 7), &[flora_colossus()], &[]);
    let colossus = cast_and_resolve(&mut engine, flora_colossus());
    assert_body(&engine, colossus, (7, 7), KeywordSet::HEXPROOF);
}

/// It counts *your* lands and follows them: opposing lands change nothing,
/// and a land played makes it a size bigger.
#[test]
fn flora_colossus_grows_with_a_land_played_and_ignores_theirs() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(
        &[flora_colossus(), forest(), forest(), forest()],
        &[forest()],
        &lots(mountain(), 2),
    );
    let colossus = on_battlefield(&engine, p0, flora_colossus()).expect("out");
    assert_eq!(pt(&engine, colossus), (3, 3), "three lands of its own");
    let _ = play_land(&mut engine, p0, forest());
    assert_eq!(pt(&engine, colossus), (4, 4), "a fourth land, a size more");
}

/// Hexproof: their Lightning Bolt cannot aim at it.
#[test]
fn flora_colossus_hexproof_keeps_their_bolt_off_it() {
    let (menu, colossus, ogre) = bolt_menu(flora_colossus(), 1);
    assert!(menu.contains(&ogre));
    assert!(!menu.contains(&colossus));
}
