//! `cards/artifacts/mv_2/jinxed_ring.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn jinxed_ring() -> CardIndex {
    card_index("29cb8f47-7bb5-4881-98ef-59fc858cf9d7")
}

/// Jinxed Ring (`29cb8f47-7bb5-4881-98ef-59fc858cf9d7`) — {2} Artifact (Implemented):
/// "Whenever a nontoken permanent is put into your graveyard from the battlefield, this artifact deals 1 damage to you.
/// Sacrifice a creature: Target opponent gains control of this artifact."
#[test]
fn jinxed_ring_deals_damage_on_nontoken_death_and_transfers_control_on_sacrifice() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                jinxed_ring(),
                index::GRIZZLY_BEARS,
                index::LLANOWAR_ELVES,
                mountain(),
            ],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 19);

    let elf = on_battlefield(&engine, P0, index::LLANOWAR_ELVES).unwrap();
    activate(&mut engine, P0, jinxed_ring(), 1);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P1],
                },
            )
            .unwrap();
    }
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
            .unwrap();
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                P0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P1],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P1, jinxed_ring()).is_some(),
        "opponent should now control Jinxed Ring"
    );
}

#[test]
fn trigger_checks_nontokens_and_ownership_including_lands() {
    let rats = card_index("c53d6d3e-64e3-48b9-afab-3bde3164888f");
    let mut e = lk::game(
        SEED,
        &[jinxed_ring(), forest(), index::GRIZZLY_BEARS],
        &[forest()],
        &[
            rats,
            lightning_bolt(),
            lightning_bolt(),
            index::STONE_RAIN,
            index::STONE_RAIN,
        ],
        &[],
    );
    lk::float(&mut e, P0, &[(ManaColor::Black, 10), (ManaColor::Red, 20)]);
    cast_with_floating(&mut e, P0, rats);
    lk::settle(&mut e);
    let token = tokens_of(&e, P0)[0];
    lk::cast_at(&mut e, P0, lightning_bolt(), &[token], &[]);
    assert_eq!(lk::life(&e, P0), 20, "token death does not count");
    let theirs = lk::obj(&e, P1, forest());
    lk::cast_at(&mut e, P0, index::STONE_RAIN, &[theirs], &[]);
    assert_eq!(
        lk::life(&e, P0),
        20,
        "enemy-owned land goes to their graveyard"
    );
    let mine = lk::obj(&e, P0, forest());
    lk::cast_at(&mut e, P0, index::STONE_RAIN, &[mine], &[]);
    assert_eq!(lk::life(&e, P0), 19, "nontoken land is a permanent too");
    let bear = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(bear)
        .unwrap()
        .controller = P1;
    e.refresh_offer();
    lk::cast_at(&mut e, P0, lightning_bolt(), &[bear], &[]);
    assert_eq!(
        lk::life(&e, P0),
        18,
        "owned creature returns to your graveyard despite enemy control"
    );
}
