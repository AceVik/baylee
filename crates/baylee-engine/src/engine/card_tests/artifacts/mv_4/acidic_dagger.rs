//! `cards/artifacts/mv_4/acidic_dagger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{ACIDIC_DAGGER, GRIZZLY_BEARS, HILL_GIANT};

#[test]
fn acidic_dagger_destroys_damaged_non_wall_and_sacrifices_when_target_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                ACIDIC_DAGGER,
                GRIZZLY_BEARS,
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dagger = on_battlefield(&engine, p0, ACIDIC_DAGGER).unwrap();
    let attacker = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let blocker = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ACIDIC_DAGGER, 0);
    aim_at(&mut engine, p0, attacker);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
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
                blockers: vec![(blocker, attacker)],
            },
        )
        .unwrap();

    // Combat damage: Grizzly Bears (2/2) dies to Hill Giant (3/3), and Acidic
    // Dagger's granted trigger destroys Hill Giant and its delayed LTB trigger
    // sacrifices Acidic Dagger.
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(blocker).unwrap().zone,
        Zone::Graveyard,
        "Hill Giant should be destroyed after taking combat damage from the targeted creature"
    );
    assert_eq!(
        engine.state().object(dagger).unwrap().zone,
        Zone::Graveyard,
        "Acidic Dagger should be sacrificed when the targeted creature leaves the battlefield this turn"
    );
}
