//! Oracle checklist: pay exactly 2 life; +2/+2 until end of turn; second activation unavailable that turn; refresh next turn and expire at end of turn.
#[allow(clippy::wildcard_imports)]
use super::*;
fn putrid_leech() -> CardIndex {
    card_index("16b723bc-aab9-40f2-8e1b-b9c2bd2af3bc")
}

#[test]
fn leech_pays_life_once_this_turn_and_its_pump_expires() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[putrid_leech()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let leech = on_battlefield(&e, p0, putrid_leech()).unwrap();
    let life = e.state().players[0].life;
    let before = pt(&e, leech);
    activate(&mut e, p0, putrid_leech(), 0);
    assert_eq!(
        e.state().players[0].life,
        life - 2,
        "life is paid as the cost"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, leech), (before.0 + 2, before.1 + 2));
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected priority");
    };
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == leech),
        "the once-each-turn limit bars a second activation"
    );
    let first_turn = e.state().turn.number;
    pass_until(&mut e, |game| {
        game.state().turn.number > first_turn
            && game.state().turn.active == p0
            && matches!(game.state().turn.phase, crate::turn::Phase::FirstMain)
            && matches!(game.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        pt(&e, leech),
        before,
        "cleanup removed the temporary pump before the next turn"
    );
    let life_after = e.state().players[0].life;
    activate(&mut e, p0, putrid_leech(), 0);
    assert_eq!(
        e.state().players[0].life,
        life_after - 2,
        "the turn limit resets on the next turn"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, leech), (before.0 + 2, before.1 + 2));
}
