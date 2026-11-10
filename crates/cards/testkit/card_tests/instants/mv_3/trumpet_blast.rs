//! `cards/instants/mv_3/trumpet_blast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn trumpet_blast() -> CardIndex {
    card_index("d3b408e7-4d39-47a3-b00c-846903f8ae77")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn trumpet_blast_pumps_only_attacking_creatures_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(),
                hill_giant(),
            ],
        )
        .battlefield(1, &[pearled_unicorn()])
        .hand(0, &[trumpet_blast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    let unicorn = on_battlefield(&engine, p1, pearled_unicorn()).unwrap();

    // Attack with Grizzly Bears only; Hill Giant stays back.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Cast Trumpet Blast during the declare attackers step.
    cast_from_hand(&mut engine, p0, trumpet_blast());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bear),
        (4, 2),
        "the attacking Grizzly Bears gets +2/+0"
    );
    assert_eq!(
        pt(&engine, giant),
        (3, 3),
        "the non-attacking Hill Giant is unaffected"
    );
    assert_eq!(
        pt(&engine, unicorn),
        (2, 2),
        "the defending creature is not attacking and is unaffected"
    );

    // Let combat finish unblocked: p1 takes 4 combat damage (20 -> 16).
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 16);

    // At the next turn's upkeep, the +2/+0 pump has expired.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    assert_eq!(pt(&engine, bear), (2, 2));
}
