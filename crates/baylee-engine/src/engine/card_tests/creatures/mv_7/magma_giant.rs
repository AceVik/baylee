//! `cards/creatures/mv_7/magma_giant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn magma_giant() -> CardIndex {
    card_index("f71d8073-d265-4387-8d08-0b9815fd2242")
}

/// Magma Giant — a 5/5 for {5}{R}{R}: "When this creature enters, it deals 2
/// damage to each creature and each player." Both Gray Ogres die, the 4/5
/// Elemental and the Giant itself are marked with 2, and each player is at 18.
#[test]
fn magma_giant_hits_every_creature_and_every_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let board = [lots(mountain(), 7), vec![gray_ogre()]].concat();
    let mut engine = table7(&board, &[magma_giant()], &[gray_ogre(), earth_elemental()]);
    let giant = cast_and_resolve(&mut engine, magma_giant());
    assert_eq!(pt(&engine, giant), (5, 5));
    assert_eq!(engine.state().object(giant).expect("out").damage, 2);
    let elemental = on_battlefield(&engine, p1, earth_elemental()).expect("survives");
    assert_eq!(engine.state().object(elemental).expect("out").damage, 2);
    assert!(
        in_graveyard(&engine, p0, gray_ogre()).is_some(),
        "mine died"
    );
    assert!(
        in_graveyard(&engine, p1, gray_ogre()).is_some(),
        "theirs died"
    );
    assert_eq!(life_of(&engine, 0), 18, "each player");
    assert_eq!(life_of(&engine, 1), 18, "each player");
}
