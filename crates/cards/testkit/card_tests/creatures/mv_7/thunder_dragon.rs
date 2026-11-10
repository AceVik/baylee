//! `cards/creatures/mv_7/thunder_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn thunder_dragon() -> CardIndex {
    card_index("70d9579f-c8a6-4406-afe1-9291fe5ee736")
}

/// Thunder Dragon — a 5/5 flier for {5}{R}{R}: "When this creature enters, it
/// deals 3 damage to each creature without flying." Both Ogres die, the 4/5
/// Elemental is marked with 3, and neither the opposing flier nor the Dragon
/// is touched.
#[test]
fn thunder_dragon_hits_every_creature_without_flying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let board = [lots(mountain(), 7), vec![gray_ogre()]].concat();
    let mut engine = table7(
        &board,
        &[thunder_dragon()],
        &[gray_ogre(), earth_elemental(), flier()],
    );
    let dragon = cast_and_resolve(&mut engine, thunder_dragon());
    assert_body(&engine, dragon, (5, 5), KeywordSet::FLYING);
    assert_eq!(engine.state().object(dragon).expect("out").damage, 0);
    assert!(
        in_graveyard(&engine, p0, gray_ogre()).is_some(),
        "mine died"
    );
    assert!(
        in_graveyard(&engine, p1, gray_ogre()).is_some(),
        "theirs died"
    );
    let elemental = on_battlefield(&engine, p1, earth_elemental()).expect("survives");
    assert_eq!(engine.state().object(elemental).expect("out").damage, 3);
    let sphinx = on_battlefield(&engine, p1, flier()).expect("out");
    assert_eq!(engine.state().object(sphinx).expect("out").damage, 0);
}
