//! Oracle: “Alliance — Whenever another creature you control enters, this
//! creature deals 1 damage to each opponent.”
#[allow(clippy::wildcard_imports)]
use super::*;

fn witty_roastmaster() -> CardIndex {
    card_index("34edbf1b-0826-49f1-a04d-15cc8c543b22")
}

/// Checklist: casting this source does not trigger its “another” clause;
/// another creature its controller casts deals one to each opponent, while
/// an opponent's own creature entry does not.
#[test]
fn witty_roastmaster_damages_opponents_for_another_controlled_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::table(SEED, mountain(), 3)
        .battlefield(0, &[mountain(), mountain(), forest(), forest(), forest()])
        .hand(0, &[witty_roastmaster(), llanowar_elves()])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .battlefield(2, &[forest()])
        .hand(2, &[llanowar_elves()])
        .life(0, 20)
        .life(1, 20)
        .life(2, 20)
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);

    cast_from_hand(&mut game, p0, witty_roastmaster());
    pass_until(&mut game, stack_is_empty);
    assert_eq!(
        game.state().players[1].life,
        20,
        "the source is not another creature"
    );
    assert_eq!(game.state().players[2].life, 20);

    cast_from_hand(&mut game, p0, llanowar_elves());
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[1].life, 19);
    assert_eq!(
        game.state().players[2].life,
        19,
        "each opponent is dealt one damage"
    );
    assert_eq!(game.state().players[0].life, 20);

    reach_their_main_phase(&mut game, p1);
    cast_from_hand(&mut game, p1, llanowar_elves());
    pass_until(&mut game, stack_is_empty);
    assert_eq!(
        game.state().players[1].life,
        19,
        "an opponent's creature is not controlled by Roastmaster's controller"
    );
    assert_eq!(game.state().players[2].life, 19);
    assert_eq!(
        game.state().players[0].life,
        20,
        "the controller is not an opponent"
    );
}
