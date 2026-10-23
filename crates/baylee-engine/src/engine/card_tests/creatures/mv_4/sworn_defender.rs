//! `cards/creatures/mv_4/sworn_defender.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn sworn_defender() -> CardIndex {
    card_index("02197d14-8d9f-4b1f-9ee0-ce8ef690843c")
}

/// Sworn Defender ({2}{W}{W} 1/3 Creature — Human Knight):
/// - "{1}: This creature's power becomes the toughness of target creature blocking or being blocked by this creature minus 1 until end of turn, and its toughness becomes 1 plus the power of that creature until end of turn."
#[test]
fn sworn_defender_sets_toughness_to_one_plus_combat_partner_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[sworn_defender(), plains()])
        .battlefield(1, &[HILL_GIANT, grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);

    let defender = on_battlefield(&engine, p0, sworn_defender()).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();
    // A creature that is neither blocking nor blocked by the Defender.
    let bystander = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    // P1 attacks with Hill Giant (3/3); Sworn Defender (1/3) blocks and activates {1} targeting Hill Giant.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p0))],
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
                blockers: vec![(defender, giant)],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, sworn_defender(), 0);
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bystander],
                    players: vec![],
                },
            )
            .is_err(),
        "a creature that is not in combat with the Defender is no target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Toughness becomes 1 + Hill Giant's power (3) = 4, so Sworn Defender survives combat!
    assert_eq!(pt(&engine, defender).1, 4);
    // The power half of the printed ability is not played: it stays 1.
    assert_eq!(pt(&engine, defender).0, 1);
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(on_battlefield(&engine, p0, sworn_defender()).is_some());
}
