//! `cards/instants/mv_3/wing_shards.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
use baylee_engine::choice::TargetPrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Wing Shards — {1}{W}{W} Instant:
/// "Target player sacrifices an attacking creature of their choice.
/// Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)"
#[test]
fn wing_shards_forces_sacrificing_only_attacking_creatures_once_per_copy() {
    let shards = card_index("b302bce5-d7ad-46d8-9dbf-8159e8709b4f");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::GRIZZLY_BEARS,
                index::HILL_GIANT,
                index::LLANOWAR_ELVES,
            ],
        )
        .hand(1, &[index::DARK_RITUAL, shards])
        .start();
    keep_mulligans(&mut engine);
    // Walk to P0's Declare Attackers on turn 3 (so P0's creatures can attack).
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let stay_back = on_battlefield(&engine, P0, index::LLANOWAR_ELVES).unwrap();

    // Advance to Declare Attackers and attack with Bears and Giant only.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1)), (giant, Defender::Player(P1))],
            },
        )
        .unwrap();

    // Wait for P1 priority during Declare Attackers, then cast Dark Ritual + Wing Shards.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    {
        let state = engine.dev_state_mut(P1).unwrap();
        state.players[1].mana_pool.add(ManaColor::Black, 1);
        state.players[1].mana_pool.add(ManaColor::White, 3);
    }
    engine.refresh_offer();

    cast_with_floating(&mut engine, P1, index::DARK_RITUAL);
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });

    cast_with_floating(&mut engine, P1, shards);
    match engine.pending().clone() {
        Pending::ChoosePlayer { player, .. } => {
            assert_eq!(player, P1);
            engine.apply(P1, PlayerAction::ChoosePlayer(P0)).unwrap();
        }
        Pending::ChooseTargets { player, .. } => {
            assert_eq!(player, P1);
            engine
                .apply(
                    P1,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![P0],
                    },
                )
                .unwrap();
        }
        other => panic!("expected player target choice for Wing Shards, got {other:?}"),
    }

    let mut sacrifices = 0usize;
    for _ in 0..40 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets {
                player,
                reason: TargetPrompt::Retarget { .. },
                ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                assert_eq!(player, P0, "target player chooses their own sacrifice");
                assert!(
                    !options.contains(&stay_back),
                    "non-attacking creature must not be on the sacrifice menu: {options:?}"
                );
                sacrifices += 1;
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected pending during Wing Shards: {other:?}"),
        }
    }
    assert_eq!(
        sacrifices, 2,
        "original + 1 Storm copy caused two sacrifices"
    );
    assert_eq!(engine.state().per_turn.spells_cast, vec![0, 2]);
    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, P0, index::HILL_GIANT).is_some());
    assert!(
        on_battlefield(&engine, P0, index::LLANOWAR_ELVES).is_some(),
        "non-attacking creature survives Wing Shards"
    );
}
