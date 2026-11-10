//! `cards/creatures/artifacts/mv_4/soldevi_golem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn soldevi_golem() -> CardIndex {
    card_index("cd444267-712c-450f-a349-e27faf4f4c7c")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn attack_unblocked(engine: &mut Engine<RegistryLookup>, player: PlayerId, attackers: &[ObjectId]) {
    let defender = if player == P0 { P1 } else { P0 };
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player: asked, .. } if *asked == player),
    );
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|&id| (id, Defender::Player(defender)))
                    .collect(),
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(defender, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
}

/// Soldevi Golem (cd444267-712c-450f-a349-e27faf4f4c7c) — {4} 5/3 Artifact Creature — Golem:
/// - "This creature doesn't untap during your untap step."
/// - "At the beginning of your upkeep, you may untap target tapped creature an opponent controls.
///   If you do, untap this creature."
#[test]
fn soldevi_golem_does_not_untap_normally_and_may_untap_with_tapped_opponent_creature_at_upkeep() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[soldevi_golem(), grizzly_bears()])
        .battlefield(1, &[hill_giant(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);

    let golem = on_battlefield(&engine, P0, soldevi_golem()).unwrap();
    let my_bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let their_giant = on_battlefield(&engine, P1, hill_giant()).unwrap();
    let their_ogre = on_battlefield(&engine, P1, gray_ogre()).unwrap();

    // Turn 1: P0 attacks with Soldevi Golem and Grizzly Bears (both become tapped).
    attack_unblocked(&mut engine, P0, &[golem, my_bears]);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(is_tapped(&engine, golem));

    // Turn 2: P1 attacks with Hill Giant (so Hill Giant is tapped, while Gray Ogre remains untapped).
    attack_unblocked(&mut engine, P1, &[their_giant]);

    // Turn 3 upkeep: Soldevi Golem did NOT untap during P0's untap step, while Grizzly Bears did.
    // Its upkeep trigger asks for a target tapped creature an opponent controls.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert!(
        is_tapped(&engine, golem),
        "Soldevi Golem does not untap during your untap step"
    );
    assert!(
        !is_tapped(&engine, my_bears),
        "ordinary creature untapped during untap step"
    );

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets for Soldevi Golem upkeep trigger");
    };
    assert!(
        options.contains(&their_giant),
        "tapped opponent creature (Hill Giant) is a legal target"
    );
    assert!(
        !options.contains(&their_ogre),
        "untapped opponent creature (Gray Ogre) is excluded"
    );
    assert!(
        !options.contains(&golem),
        "your own tapped creature is excluded"
    );

    // 1. First upkeep (turn 3): target Hill Giant, then decline the 'may' -> neither untaps.
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![their_giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert!(
        is_tapped(&engine, their_giant) && is_tapped(&engine, golem),
        "when declining the 'may', neither target nor Soldevi Golem untaps"
    );

    // Turn 4: P1 attacks with Hill Giant again so it stays tapped entering turn 5.
    attack_unblocked(&mut engine, P1, &[their_giant]);

    // 2. Second upkeep (turn 5): target Hill Giant, then accept the 'may' -> both untap!
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5 && matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![their_giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.number == 5 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert!(
        !is_tapped(&engine, their_giant) && !is_tapped(&engine, golem),
        "when accepting the 'may', both the opponent's tapped creature and Soldevi Golem untap"
    );
}
