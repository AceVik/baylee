//! `cards/artifacts/mv_4/grinning_totem.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRINNING_TOTEM;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Grinning Totem (ef0bf01c-8146-4b02-b9b2-61b60c7b402a) — {4} Artifact:
/// "{2}, {T}, Sacrifice this artifact: Search target opponent's library for a
/// card and exile it. Then that player shuffles..."
#[test]
fn grinning_totem_activates_to_shuffle_opponent_library() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[GRINNING_TOTEM, plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, GRINNING_TOTEM, 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, GRINNING_TOTEM).is_none());
}
