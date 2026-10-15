//! `cards/creatures/mv_8/gigantomancer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn gigantomancer() -> CardIndex {
    card_index("c4b5567a-f195-4638-94b5-0b933dba1850")
}

/// Gigantomancer is a 1/1 for {7}{G}.
#[test]
fn gigantomancer_is_cast_for_eight_as_a_one_one() {
    let mut engine = at_main(&[forest(); 8], &[gigantomancer()]);
    let mage = cast_and_settle(&mut engine, gigantomancer());
    assert_eq!(pt(&engine, mage), (1, 1));
}

/// "{1}: Target creature you control has base power and toughness 7/7 until
/// end of turn": aimed at the controller's Bears the Bears become 7/7 and the
/// Gigantomancer stays a 1/1; the opponent's creature is not on the menu.
#[test]
fn gigantomancer_makes_the_targeted_creature_seven_seven() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = at_main(&[gigantomancer(), grizzly_bears(), forest()], &[]);
    let mage = on_battlefield(&engine, p0, gigantomancer()).expect("the mage");
    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("my bear");
    let _ = p1;
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, gigantomancer(), 0);
    let menu = aim_at(&mut engine, p0, bear);
    assert!(menu.contains(&mage) && menu.contains(&bear), "{menu:?}");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (7, 7), "the target");
    assert_eq!(pt(&engine, mage), (1, 1), "not the source");
}

/// The menu holds only creatures its controller has.
#[test]
fn gigantomancer_cannot_aim_at_an_opponents_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gigantomancer(), forest()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let theirs = on_battlefield(&engine, p1, grizzly_bears()).expect("their bear");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, gigantomancer(), 0);
    let mage = on_battlefield(&engine, p0, gigantomancer()).expect("the mage");
    let menu = aim_at(&mut engine, p0, mage);
    assert!(!menu.contains(&theirs), "{menu:?}");
}
