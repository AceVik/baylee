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

/// "It deals 2 damage to each attacking creature and each blocking creature": the other attacker
/// (never blocked) and the blocker die too, while the 1/1s that stayed out of combat are
/// untouched; and the opponent takes nothing, because the other attacker died before damage.
#[test]
fn goblin_swine_rider_hits_every_attacker_and_blocker_but_nobody_else() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_swine_rider(), gray_ogre(), llanowar_elves()])
        .battlefield(1, &[grizzly_bears(), festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rider = on_battlefield(&engine, p0, goblin_swine_rider()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(rider, Defender::Player(p1)), (ogre, Defender::Player(p1))],
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
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });

    assert_eq!(
        engine.state().object(ogre).unwrap().zone,
        Zone::Graveyard,
        "a 2/2 attacker that nobody blocked takes the 2 damage"
    );
    assert_eq!(engine.state().object(bear).unwrap().zone, Zone::Graveyard);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "a creature that did not attack is not hit"
    );
    assert!(
        on_battlefield(&engine, p1, festering_goblin()).is_some(),
        "a creature that did not block is not hit"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the dead Ogre dealt nothing"
    );
}
