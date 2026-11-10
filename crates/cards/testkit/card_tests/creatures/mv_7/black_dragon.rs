//! `cards/creatures/mv_7/black_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn black_dragon() -> CardIndex {
    card_index("ad804712-18ba-49d7-9af2-4361edaa6cf8")
}

/// Black Dragon — a 4/4 flier for {5}{B}{B} whose Acid Breath gives target
/// creature an opponent controls -3/-3 until end of turn. The menu is the
/// opponent's creatures only (the Dragon controller's own Ogre is not on it);
/// a 2/2 Ogre pointed at dies, and a bigger target is only shrunk.
#[test]
fn black_dragon_acid_breath_kills_a_small_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let board = [lots(swamp(), 7), vec![gray_ogre()]].concat();
    let mut engine = table7(&board, &[black_dragon()], &[gray_ogre()]);
    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    pay_exactly_and_cast(&mut engine, black_dragon());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&theirs), "an opposing creature");
    assert!(!options.contains(&mine), "not one's own creature");
    point_at(&mut engine, p0, &[theirs], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, gray_ogre()).is_some(),
        "-3/-3 kills a 2/2"
    );
    let dragon = on_battlefield(&engine, p0, black_dragon()).expect("out");
    assert_body(&engine, dragon, (4, 4), KeywordSet::FLYING);
}

#[test]
fn black_dragon_acid_breath_shrinks_a_big_creature_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&lots(swamp(), 7), &[black_dragon()], &[earth_elemental()]);
    let elemental = on_battlefield(&engine, p1, earth_elemental()).expect("out");
    assert_eq!(pt(&engine, elemental), (4, 5));
    pay_exactly_and_cast(&mut engine, black_dragon());
    let _ = pass_until_targets(&mut engine, p0);
    point_at(&mut engine, p0, &[elemental], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elemental), (1, 2), "-3/-3 until end of turn");
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert_eq!(pt(&engine, elemental), (4, 5), "and then it wears off");
}
