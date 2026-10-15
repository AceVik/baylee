//! `cards/creatures/mv_8/pestilence_demon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn pestilence_demon() -> CardIndex {
    card_index("2572add6-c0b1-44bd-8713-1e94e3fe69d6")
}

/// Pestilence Demon is a 7/6 flier for {5}{B}{B}{B}.
#[test]
fn pestilence_demon_is_cast_for_eight_with_flying() {
    let mut engine = at_main(&[swamp(); 8], &[pestilence_demon()]);
    let demon = cast_and_settle(&mut engine, pestilence_demon());
    assert_eq!(pt(&engine, demon), (7, 6));
    assert!(keywords(&engine, demon).contains(KeywordSet::FLYING));
}

/// "{B}: This creature deals 1 damage to each creature and each player":
/// one activation kills the 1/1 Elves, marks the Bears and the Demon itself
/// and costs both players 1 life; a second kills the Bears.
#[test]
fn pestilence_demon_pings_every_creature_and_every_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[pestilence_demon(), swamp(), swamp()])
        .battlefield(1, &[llanowar_elves(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    let demon = on_battlefield(&engine, p0, pestilence_demon()).expect("the Demon");
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("the bear");
    let (mine, theirs) = (life_of(&engine, 0), life_of(&engine, 1));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, pestilence_demon(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the 1/1 dies"
    );
    assert_eq!(engine.state().object(bear).unwrap().damage, 1);
    assert_eq!(engine.state().object(demon).unwrap().damage, 1, "and itself");
    assert_eq!(life_of(&engine, 0), mine - 1, "its controller too");
    assert_eq!(life_of(&engine, 1), theirs - 1);

    activate(&mut engine, p0, pestilence_demon(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert_eq!(life_of(&engine, 0), mine - 2);
    assert_eq!(life_of(&engine, 1), theirs - 2);
}
