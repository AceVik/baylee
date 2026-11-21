//! `cards/instants/mv_2/wicked_reward.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "7aad0a80-cf7b-4224-8e72-d611e0be3383"
fn wicked_reward() -> CardIndex {
    card_index("7aad0a80-cf7b-4224-8e72-d611e0be3383")
}

/// Wicked Reward ({1}{B}): cannot be cast without a creature you control to
/// sacrifice (opponent's creatures do not qualify).
#[test]
fn wicked_reward_requires_a_controlled_creature_to_sacrifice() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[wicked_reward()], &[]);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    assert!(
        !lk::castable(&engine, P0, wicked_reward()),
        "cannot sacrifice an opponent's creature"
    );
}

/// Sacrificing a creature you control gives target creature +4/+2 until end
/// of turn, and the bonus wears off next turn.
#[test]
fn wicked_reward_sacrifices_creature_and_gives_plus_four_plus_two_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[index::LLANOWAR_ELVES, index::GRIZZLY_BEARS],
        &[],
        &[wicked_reward()],
        &[],
    );
    let elf = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut engine, P0, wicked_reward());
    for _ in 0..6 {
        match engine.pending().clone() {
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseTargets {
                            objects: vec![bears],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { .. } => {
                engine
                    .apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    lk::settle(&mut engine);

    assert!(
        in_graveyard(&engine, P0, index::LLANOWAR_ELVES).is_some(),
        "sacrificed creature went to graveyard"
    );
    assert_eq!(pt(&engine, bears), (6, 4), "+4/+2 until end of turn");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2), "+4/+2 wears off at end of turn");
}
