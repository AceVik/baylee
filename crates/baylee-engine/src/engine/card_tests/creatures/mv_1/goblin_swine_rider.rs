//! `cards/creatures/mv_1/goblin_swine_rider.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn goblin_swine_rider() -> CardIndex {
    card_index("9ebe32fc-3b6e-4cb9-9af0-1a6c3c6ad885")
}

/// `Goblin Swine-Rider` ({R}, 1/1 Goblin, `Coverage::Implemented`):
/// `Whenever this creature becomes blocked, it deals 2 damage to each attacking
/// creature and each blocking creature.`
#[test]
fn goblin_swine_rider_deals_two_damage_to_each_attacking_and_blocking_creature_when_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[goblin_swine_rider()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, goblin_swine_rider());
    pass_until(&mut engine, stack_is_empty);
    let rider = on_battlefield(&engine, p0, goblin_swine_rider()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    // Advance to P0's next turn so Goblin Swine-Rider can attack.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(rider, Defender::Player(p1))],
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
                blockers: vec![(bear, rider)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(rider).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Graveyard);
}
