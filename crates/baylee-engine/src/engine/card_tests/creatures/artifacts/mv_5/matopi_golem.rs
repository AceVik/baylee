//! `cards/creatures/artifacts/mv_5/matopi_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn matopi_golem() -> CardIndex {
    card_index("0308b9f2-11da-41d0-8d18-18e8c0c600ab")
}

/// Partial Matopi Golem retains its 3/3 body. The optional regeneration is
/// omitted together with the unsupported counter rider when a shield is used.
#[test]
fn matopi_golem_attacks_for_three_without_the_unsupported_regeneration() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[matopi_golem()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, matopi_golem());
    pass_until(&mut engine, stack_is_empty);
    let golem = on_battlefield(&engine, p0, matopi_golem()).unwrap();

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == golem),
        "regeneration without its required counter rider is not offered"
    );
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(golem, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "the body deals 3 damage"
    );
    assert_eq!(engine.state().players[0].life, 20);
}
