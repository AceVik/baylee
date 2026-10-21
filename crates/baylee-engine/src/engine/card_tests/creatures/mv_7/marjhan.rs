//! Marjhan — `{5}{U}{U}` 8/8 Creature — Serpent (Homelands)
//!
//! This creature doesn't untap during your untap step.
//! `{U}{U}`, Sacrifice a creature: Untap this creature. Activate only during
//! your upkeep.
//! This creature can't attack unless defending player controls an Island.
//! `{U}{U}`: This creature gets -1/-0 until end of turn and deals 1 damage to
//! target attacking creature without flying.
//! When you control no Islands, sacrifice this creature.

#[allow(clippy::wildcard_imports)]
use super::*;

fn marjhan() -> CardIndex {
    card_index("3fdee2ab-7ec6-4fc6-ad99-f04571f94583")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn marjhan_shrinks_to_ping_attacking_nonflyer_and_sacrifices_without_islands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[bears()])
        .battlefield(1, &[marjhan(), island(), island()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let bear = on_battlefield(&engine, p0, bears()).unwrap();
    let serpent = on_battlefield(&engine, p1, marjhan()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1))],
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // P1 activates Marjhan's {U}{U} ability (ability_index: 3) targeting attacking Grizzly Bears.
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: serpent,
                ability_index: 3,
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert_eq!(pt(&engine, serpent), (7, 8));
    assert_eq!(engine.state().object(bear).unwrap().damage, 1);

    // A Marjhan controller with no Islands sacrifices Marjhan via state trigger.
    let mut engine2 = Duel::new(SEED, forest())
        .battlefield(0, &[marjhan()])
        .start();
    keep_mulligans(&mut engine2);
    reach_main_phase(&mut engine2, p0);
    assert!(on_battlefield(&engine2, p0, marjhan()).is_none());
    assert!(in_graveyard(&engine2, p0, marjhan()).is_some());
}
