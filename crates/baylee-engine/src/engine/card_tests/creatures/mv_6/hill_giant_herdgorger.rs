// Oracle checklist: When this creature enters, you gain 3 life.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("05c5752a-6ccb-45ff-bb85-5f89efd5944b")
}

// A real entry gains exactly three once; the opponent's life stays put.
#[test]
fn entry_gains_three_life_for_its_controller() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .life(0, 11)
        .life(1, 16)
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    cast_from_hand(&mut game, p0, subject());
    pass_until(&mut game, stack_is_empty);
    assert!(on_battlefield(&game, p0, subject()).is_some());
    assert_eq!(game.state().players[0].life, 14);
    assert_eq!(game.state().players[1].life, 16);
}
