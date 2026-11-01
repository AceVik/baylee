//! Warning — `{W}` Instant (Ice Age)
//!
//! Prevent all combat damage that would be dealt by target attacking creature
//! this turn.
//! (`Coverage::Implemented`: `Modifier::PreventDamageFromIt` is combat damage
//! only, which is what is printed.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn warning() -> CardIndex {
    card_index("c5ba0f0f-65c5-4ffa-987a-f320b401ec8f")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn warning_targets_only_attacking_creature_and_prevents_its_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[warning()])
        .battlefield(0, &[plains(), grizzly_bears()])
        .battlefield(1, &[hill_giant(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    let our_blocker = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let attacker = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let non_attacker = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let spell = in_hand(&engine, p0, warning()).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(our_blocker, attacker)],
            },
        )
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    // Only the attacking Hill Giant is a legal target; non-attacking creatures are excluded.
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!(options, &[attacker]);
    assert!(!options.contains(&non_attacker));
    assert!(!options.contains(&our_blocker));

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![attacker],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(our_blocker).unwrap().zone,
        Zone::Battlefield,
        "Grizzly Bears survives because Warning prevented Hill Giant's 3 combat damage"
    );
    assert_eq!(
        engine.state().object(attacker).unwrap().damage,
        2,
        "Grizzly Bears still dealt its 2 combat damage to Hill Giant"
    );
}
