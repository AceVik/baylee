//! `cards/enchantments/mv_3/circle_of_despair.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn circle_of_despair() -> CardIndex {
    card_index("4ad57fbf-69f3-4367-b061-ed8e0b83b9af")
}

fn target_yourself(engine: &mut Engine<RegistryLookup>) {
    assert!(
        matches!(engine.pending(), Pending::ChooseTargets { player, .. } if *player == P0),
        "the supported self-only subset still requires a real target"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![]
                }
            )
            .is_err(),
        "the target is mandatory"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P1]
                }
            )
            .is_err(),
        "other recipients remain unsupported"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .unwrap();
}

/// Circle of Despair (4ad57fbf-69f3-4367-b061-ed8e0b83b9af) — {1}{W}{B} Enchantment:
/// "{1}, Sacrifice a creature: The next time a source of your choice would deal
/// damage to any target this turn, prevent that damage."
#[test]
fn circle_of_despair_sacrifices_creature_to_prevent_next_damage_from_chosen_source() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[circle_of_despair(), plains(), GRIZZLY_BEARS])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    // Advance to P1's combat and attack with Hill Giant.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
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
        .apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // P1 gets priority in DeclareBlockers first; pass to P0.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, circle_of_despair(), 0);
    target_yourself(&mut engine);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseDamageSource { player, .. } if *player == P0),
    );
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    let chosen = *options.iter().find(|s| s.object == giant).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: chosen,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(on_battlefield(&engine, P0, GRIZZLY_BEARS).is_none());
    assert_eq!(engine.state().players[0].life, 20);
}

/// "a source of your choice": with two attackers, the chosen one (Hill Giant)
/// is prevented and the other (the opponent's Grizzly Bears) still deals its
/// damage.
#[test]
fn circle_of_despair_lets_damage_from_another_source_through() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[circle_of_despair(), plains(), GRIZZLY_BEARS])
        .battlefield(1, &[HILL_GIANT, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();
    let their_bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (giant, Defender::Player(P0)),
                    (their_bears, Defender::Player(P0)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, circle_of_despair(), 0);
    target_yourself(&mut engine);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseDamageSource { player, .. } if *player == P0),
    );
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    let chosen = *options.iter().find(|s| s.object == giant).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseDamageSource {
                choice,
                source: chosen,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[0].life, 18, "only the Bears' 2");
}
