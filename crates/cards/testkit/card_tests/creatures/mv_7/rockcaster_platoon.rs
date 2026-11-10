//! `cards/creatures/mv_7/rockcaster_platoon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn rockcaster_platoon() -> CardIndex {
    card_index("3bc82914-1524-4d57-8b38-9788867a4046")
}

/// Rockcaster Platoon — a 5/7 for {5}{W}{W}.
#[test]
fn rockcaster_platoon_costs_seven_and_is_a_five_seven() {
    let mut engine = table7(&lots(plains(), 7), &[rockcaster_platoon()], &[]);
    let platoon = cast_and_resolve(&mut engine, rockcaster_platoon());
    assert_body(&engine, platoon, (5, 7), KeywordSet::default());
}

/// "{4}{G}: This creature deals 2 damage to each creature with flying and each
/// player." The opposing flier is marked with 2, the ground Ogre is not, and
/// both players are at 18.
#[test]
fn rockcaster_platoon_hits_fliers_and_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = lots(forest(), 5);
    board.push(rockcaster_platoon());
    let mut engine = table7(&board, &[], &[flier(), gray_ogre()]);
    let sphinx = on_battlefield(&engine, p1, flier()).expect("out");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("out");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, rockcaster_platoon(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(sphinx).expect("out").damage, 2);
    assert_eq!(engine.state().object(ogre).expect("out").damage, 0);
    assert_eq!(life_of(&engine, 0), 18);
    assert_eq!(life_of(&engine, 1), 18);
}
