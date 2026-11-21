//! `cards/artifacts/mv_2/dolmen_gate.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn dolmen_gate() -> CardIndex {
    card_index("d53d000f-c92f-4d70-ae60-8be3791da8a2")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Dolmen Gate (d53d000f-c92f-4d70-ae60-8be3791da8a2) — {2} Artifact:
/// "Prevent all combat damage that would be dealt to attacking creatures you
/// control."
///
/// An attacking creature you control takes 0 combat damage from a lethal
/// blocker while still dealing its own combat damage to that blocker.
#[test]
fn dolmen_gate_prevents_combat_damage_to_your_attacking_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dolmen_gate(), grizzly_bears()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .expect("attack with Grizzly Bears");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, bears)],
            },
        )
        .expect("block with 3/3 Hill Giant");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_some(),
        "Dolmen Gate prevents the 3 combat damage from Hill Giant"
    );
    assert_eq!(
        engine.state().object(bears).unwrap().damage,
        0,
        "0 combat damage marked on the attacking Grizzly Bears"
    );
    assert_eq!(
        engine.state().object(giant).unwrap().damage,
        2,
        "attacking Grizzly Bears still dealt its 2 combat damage to Hill Giant"
    );
}

/// Noncombat damage to an attacking creature is NOT prevented, and combat
/// damage to your blocking creature or opponent's attacking creature is NOT
/// prevented.
#[test]
fn dolmen_gate_does_not_prevent_noncombat_or_blocking_or_opponent_attacking_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dolmen_gate(), hill_giant()])
        .battlefield(1, &[mountain(), grizzly_bears()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // 1) Noncombat Lightning Bolt kills p0's attacking 3/3 Hill Giant.
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .unwrap();
    // Pass p0's priority after DeclareAttackers so p1 can cast Lightning Bolt.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let bolt = in_hand(&engine, p1, lightning_bolt()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, hill_giant()).is_some(),
        "noncombat damage is NOT prevented by Dolmen Gate"
    );

    // 2) On p1's turn, p1's attacking 2/2 and p0's blocking 2/2 BOTH take combat damage.
    let mut engine2 = Duel::new(SEED, forest())
        .battlefield(0, &[dolmen_gate(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine2);
    reach_their_main_phase(&mut engine2, p1);
    let p0_bears = on_battlefield(&engine2, p0, grizzly_bears()).unwrap();
    let p1_bears = on_battlefield(&engine2, p1, grizzly_bears()).unwrap();
    pass_until(&mut engine2, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine2
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(p1_bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine2, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine2
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(p0_bears, p1_bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine2, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(
        in_graveyard(&engine2, p0, grizzly_bears()).is_some(),
        "p0's blocking creature is NOT protected"
    );
    assert!(
        in_graveyard(&engine2, p1, grizzly_bears()).is_some(),
        "p1's attacking creature is NOT protected by p0's Dolmen Gate"
    );
}
