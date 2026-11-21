//! `cards/instants/mv_2/shrapnel_blast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "d3c623f3-c7c9-47c0-8096-6dee3267baff"
fn shrapnel_blast() -> CardIndex {
    card_index("d3c623f3-c7c9-47c0-8096-6dee3267baff")
}

/// Shrapnel Blast ({1}{R}): cannot be cast without an artifact you control
/// (opponent's artifacts do not qualify).
#[test]
fn shrapnel_blast_requires_a_controlled_artifact_to_sacrifice() {
    let mut engine = lk::game(SEED, &[], &[sol_ring()], &[shrapnel_blast()], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::Red, 2)]);
    assert!(
        !lk::castable(&engine, P0, shrapnel_blast()),
        "cannot sacrifice opponent's artifact"
    );
}

/// Sacrificing a controlled artifact deals 5 damage to any target (player or
/// creature).
#[test]
fn shrapnel_blast_sacrifices_artifact_and_deals_five_damage() {
    let mut engine = lk::game(
        SEED,
        &[sol_ring(), index::ORNITHOPTER],
        &[index::SERRA_ANGEL],
        &[shrapnel_blast(), shrapnel_blast()],
        &[],
    );
    let ring = lk::obj(&engine, P0, sol_ring());
    let thopter = lk::obj(&engine, P0, index::ORNITHOPTER);
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);

    // 1. Sacrifice Sol Ring -> 5 damage to opponent.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut engine, P0, shrapnel_blast());
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseTargets { .. } => lk::aim(&mut engine, &[], &[P1]),
            Pending::ChooseCards { .. } => engine
                .apply(
                    P0,
                    PlayerAction::ChooseObjects {
                        objects: vec![ring],
                    },
                )
                .unwrap(),
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, sol_ring()).is_some());
    assert_eq!(lk::life(&engine, P1), 15, "5 damage to opponent");

    // 2. Sacrifice Ornithopter -> 5 damage kills 4/4 Serra Angel.
    cast_with_floating(&mut engine, P0, shrapnel_blast());
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseTargets { .. } => lk::aim(&mut engine, &[angel], &[]),
            Pending::ChooseCards { .. } => engine
                .apply(
                    P0,
                    PlayerAction::ChooseObjects {
                        objects: vec![thopter],
                    },
                )
                .unwrap(),
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P0, index::ORNITHOPTER).is_some());
    assert!(in_graveyard(&engine, P1, index::SERRA_ANGEL).is_some());
}
