//! `cards/instants/mv_3/reaping_the_graves.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
use baylee_engine::choice::TargetPrompt;
use baylee_engine::event::Cause;
use baylee_engine::zone::ZonePosition;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Reaping the Graves — {2}{B} Instant:
/// "Return target creature card from your graveyard to your hand.
/// Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)"
#[test]
fn reaping_the_graves_returns_multiple_own_graveyard_creatures_and_excludes_others() {
    let reaping = card_index("80c663a5-2794-4748-b47a-34352fb2112f");
    let mut engine = Duel::new(SEED, forest())
        .hand(
            0,
            &[
                index::DARK_RITUAL,
                reaping,
                index::GRIZZLY_BEARS,
                index::HILL_GIANT,
                index::LIGHTNING_BOLT,
            ],
        )
        .hand(1, &[index::LLANOWAR_ELVES])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Seed two creature cards and one instant in P0's graveyard, plus a creature in P1's graveyard.
    let bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = in_hand(&engine, P0, index::HILL_GIANT).unwrap();
    let bolt = in_hand(&engine, P0, index::LIGHTNING_BOLT).unwrap();
    let opp_elf = in_hand(&engine, P1, index::LLANOWAR_ELVES).unwrap();
    {
        let state = engine.dev_state_mut(P0).unwrap();
        for card in [bears, giant, bolt] {
            state
                .move_object(
                    card,
                    ZoneLocation::Graveyard(P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        state
            .move_object(
                opp_elf,
                ZoneLocation::Graveyard(P1),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        state.players[0].mana_pool.add(ManaColor::Black, 4);
    }
    engine.refresh_offer();

    // Cast 1 prior spell so Storm creates 1 copy.
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, reaping);
    let options = pass_until_targets(&mut engine, P0);
    assert!(options.contains(&bears) && options.contains(&giant));
    assert!(
        !options.contains(&bolt),
        "noncreature card in your graveyard cannot be targeted: {options:?}"
    );
    assert!(
        !options.contains(&opp_elf),
        "opponent's graveyard creature cannot be targeted: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();

    // Retarget the Storm copy to Hill Giant.
    let mut retargeted = 0usize;
    for _ in 0..30 {
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
                retargeted += 1;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![giant],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected pending during Reaping the Graves: {other:?}"),
        }
    }
    assert_eq!(retargeted, 1);
    assert_eq!(engine.state().per_turn.spells_cast, vec![2, 0]);
    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(in_hand(&engine, P0, index::HILL_GIANT).is_some());
    assert!(in_graveyard(&engine, P0, index::LIGHTNING_BOLT).is_some());
    assert!(in_graveyard(&engine, P1, index::LLANOWAR_ELVES).is_some());
}
