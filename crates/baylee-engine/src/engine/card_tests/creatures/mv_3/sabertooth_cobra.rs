//! `cards/creatures/mv_3/sabertooth_cobra.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sabertooth_cobra() -> CardIndex {
    card_index("115c7ca7-c27b-43ca-9281-be6913e90059")
}

/// Sabertooth Cobra (`{2}{G}`, `Coverage::Partial`): 2/2 Snake that gives a
/// player a poison counter whenever it deals damage to that player.
#[test]
fn sabertooth_cobra_gives_damaged_player_a_poison_counter() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[sabertooth_cobra()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sabertooth_cobra());
    pass_until(&mut engine, stack_is_empty);
    let cobra = on_battlefield(&mut engine, p0, sabertooth_cobra()).unwrap();

    // Advance to p0's next turn so Sabertooth Cobra can attack.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(cobra, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[1].poison, 1);
}
