//! `cards/sorceries/mv_3/dream_cache.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::DREAM_CACHE;

const P0: PlayerId = PlayerId::new(0);

/// Dream Cache (81338d64-8fc3-41ac-afc8-e2a79df1159b) — {2}{U} Sorcery:
/// "Draw three cards, then put two cards from your hand both on top or both on
/// the bottom of your library."
#[test]
fn dream_cache_draws_three_cards_and_returns_two_from_hand_to_library() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[DREAM_CACHE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, DREAM_CACHE);

    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player, options, ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0], options[1]],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before // -1 Dream Cache cast, +3 drawn, -2 returned to library
    );
}
