//! `cards/creatures/mv_6/champion_lancer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn champion_lancer() -> CardIndex {
    card_index("2681bfb5-127b-4046-85ac-72be584999cb")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn mogg_fanatic() -> CardIndex {
    card_index("7efd5d62-6da2-428a-98e3-5b842f668656")
}

/// Champion Lancer (2681bfb5-127b-4046-85ac-72be584999cb) — `{4}{W}{W}` 3/3
/// Creature — Human Knight:
/// "Prevent all damage that would be dealt to this creature by creatures."
#[test]
fn champion_lancer_prevents_combat_and_ability_damage_from_creatures_only() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .battlefield(1, &[hill_giant(), mogg_fanatic(), mountain()])
        .hand(0, &[champion_lancer()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, champion_lancer());
    pass_until(&mut engine, stack_is_empty);

    let lancer = on_battlefield(&engine, P0, champion_lancer()).expect("Champion Lancer entered");
    let giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    assert_eq!(pt(&engine, lancer), (3, 3));

    // P1's turn: 3/3 Hill Giant attacks; Champion Lancer blocks, prevents all 3 combat damage,
    // and kills Hill Giant.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(lancer, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(in_graveyard(&engine, P1, hill_giant()).is_some());
    assert_eq!(
        engine.state().object(lancer).unwrap().damage,
        0,
        "combat damage from Hill Giant is prevented"
    );

    // Noncombat damage from a creature ability (Mogg Fanatic sacrificing itself) is also prevented.
    activate(&mut engine, P1, mogg_fanatic(), 0);
    aim_at(&mut engine, P1, lancer);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(lancer).unwrap().damage,
        0,
        "noncombat damage from a creature source (via LKI) is prevented"
    );

    // Noncreature damage (Lightning Bolt) is NOT prevented and destroys Champion Lancer.
    cast_from_hand(&mut engine, P1, lightning_bolt());
    aim_at(&mut engine, P1, lancer);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P0, champion_lancer()).is_some());
}
