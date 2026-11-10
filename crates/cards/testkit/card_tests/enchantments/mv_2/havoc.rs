// Oracle checklist: Whenever an opponent casts a white spell, they lose 2 life.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("d95aeeb0-b722-4f70-85fc-6866be4854bd")
}

// Positive: opponent casts matching colored spell; negative: own cast or different color.
#[test]
fn opposing_matching_spell_triggers_exact_life_change() {
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[subject()])
        .battlefield(1, &[mountain(), mountain(), mountain(), plains()])
        .hand(1, &[serra_zealot()])
        .start();
    keep_mulligans(&mut game);
    reach_their_main_phase(&mut game, p1);
    assert_eq!(game.state().players[0].life, 20);
    assert_eq!(game.state().players[1].life, 20);
    cast_from_hand(&mut game, p1, serra_zealot());
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 20);
    assert_eq!(game.state().players[1].life, 18);
}

#[test]
fn own_matching_cast_and_opponent_wrong_color_do_not_trigger() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut own = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), plains(), plains(), plains()])
        .hand(0, &[serra_zealot()])
        .start();
    keep_mulligans(&mut own);
    assert!(walk_to_own_main(&mut own, p0));
    cast_from_hand(&mut own, p0, serra_zealot());
    pass_until(&mut own, stack_is_empty);
    assert_eq!(own.state().players[0].life, 20);
    assert_eq!(own.state().players[1].life, 20);

    let mut wrong = Duel::new(SEED, forest())
        .battlefield(0, &[subject()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .hand(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut wrong);
    reach_their_main_phase(&mut wrong, p1);
    cast_from_hand(&mut wrong, p1, gray_ogre());
    pass_until(&mut wrong, stack_is_empty);
    assert_eq!(wrong.state().players[0].life, 20);
    assert_eq!(wrong.state().players[1].life, 20);
}
