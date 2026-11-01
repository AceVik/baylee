//! `cards/creatures/mv_2/swamp_mosquito.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn swamp_mosquito() -> CardIndex {
    card_index("4bb4844c-2678-45c7-8f5e-9cf185fd484b")
}

/// Swamp Mosquito ({1}{B} 0/1 Creature — Insect):
/// - Flying
/// - "Whenever this creature attacks and isn't blocked, defending player gets a poison counter."
#[test]
fn swamp_mosquito_flies_and_gives_defending_player_a_poison_counter_when_unblocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp_mosquito()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mosquito = on_battlefield(&engine, p0, swamp_mosquito()).unwrap();
    assert!(keywords(&engine, mosquito).contains(KeywordSet::FLYING));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mosquito, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[1].poison, 1);
}

/// "...attacks and isn't blocked": a blocked Swamp Mosquito gives no counter.
#[test]
fn swamp_mosquito_gives_no_poison_counter_when_it_is_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp_mosquito()])
        .battlefield(1, &[wall_of_air()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mosquito = on_battlefield(&engine, p0, swamp_mosquito()).unwrap();
    let wall = on_battlefield(&engine, p1, wall_of_air()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mosquito, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, mosquito)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].poison, 0);
}
