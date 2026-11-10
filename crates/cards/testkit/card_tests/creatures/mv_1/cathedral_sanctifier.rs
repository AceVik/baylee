#[allow(clippy::wildcard_imports)]
use super::*;

fn cathedral_sanctifier_card() -> CardIndex {
    card_index("1050f029-60d4-4769-b39f-9100c949a822")
}

/// Checklist: entering causes controller to gain exactly 3 after entry; it is
/// a 1/1; opponent's life does not change.
#[test]
fn cathedral_sanctifier_gains_three_life_on_entry() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .life(0, 17)
        .life(1, 24)
        .battlefield(0, &[plains()])
        .hand(0, &[cathedral_sanctifier_card()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    cast_from_hand(&mut game, p0, cathedral_sanctifier_card());
    assert_eq!(
        game.state().players[0].life,
        17,
        "trigger waits for entry and resolution"
    );
    pass_until(&mut game, stack_is_empty);
    let sanctifier = on_battlefield(&game, p0, cathedral_sanctifier_card()).unwrap();
    assert_eq!(pt(&game, sanctifier), (1, 1));
    assert_eq!(game.state().players[0].life, 20);
    assert_eq!(game.state().players[1].life, 24);
}
