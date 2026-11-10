//! `cards/creatures/mv_7/angel_of_despair.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn angel_of_despair() -> CardIndex {
    card_index("02520741-d7f0-46e6-acbe-010b65fc24c0")
}

/// Angel of Despair — "Flying" and "When this creature enters, destroy
/// target permanent." Cast off exactly {3}{W}{W}{B}{B}; the trigger asks for
/// a target on any permanent (the Angel itself and a land of its own
/// controller are on the menu with the opponent's creature) and destroys the
/// one chosen.
#[test]
fn angel_of_despair_destroys_the_permanent_it_points_at() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let board = [lots(plains(), 2), lots(swamp(), 2), lots(forest(), 3)].concat();
    let mut engine = table7(&board, &[angel_of_despair()], &[gray_ogre()]);
    pay_exactly_and_cast(&mut engine, angel_of_despair());
    let options = pass_until_targets(&mut engine, p0);

    let angel = on_battlefield(&engine, p0, angel_of_despair()).expect("the Angel resolved");
    assert_body(&engine, angel, (5, 5), KeywordSet::FLYING);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("the Ogre is out");
    assert!(
        options.contains(&ogre),
        "an opposing creature is a permanent"
    );
    assert!(
        options.contains(&lands_of(&engine, p0)[0]),
        "so is one's own land: \"target permanent\" is any permanent"
    );
    assert!(options.contains(&angel), "and the Angel itself");

    point_at(&mut engine, p0, &[ogre], &[]);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, gray_ogre()).is_some(),
        "the targeted Ogre was destroyed"
    );
    assert!(
        on_battlefield(&engine, p0, angel_of_despair()).is_some(),
        "the Angel was not"
    );
}
