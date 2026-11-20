//! `cards/instants/mv_3/sprouting_vines.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

/// Sprouting Vines — {2}{G} Instant:
/// "Search your library for a basic land card, reveal that card, put it into your hand, then shuffle.
/// Storm (When you cast this spell, copy it for each spell cast before it this turn.)"
#[test]
fn sprouting_vines_fetches_one_basic_land_per_copy_and_excludes_nonbasics() {
    let vines = card_index("a38ad1aa-fc12-49be-b9f7-fe287597a5ca");
    let mut engine = Duel::new(SEED, index::GRIZZLY_BEARS)
        .hand(
            0,
            &[index::DARK_RITUAL, vines, forest(), island(), badlands()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let basic_forest = hand_to_library_top(&mut engine, P0, forest());
    let basic_island = hand_to_library_top(&mut engine, P0, island());
    let nonbasic = hand_to_library_top(&mut engine, P0, badlands());

    {
        let state = engine.dev_state_mut(P0).unwrap();
        state.players[0].mana_pool.add(ManaColor::Black, 1);
        state.players[0].mana_pool.add(ManaColor::Green, 3);
    }
    engine.refresh_offer();

    // 1 prior spell -> Storm creates 1 copy -> 2 total basic land searches.
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, vines);
    let mut searches = 0usize;
    for _ in 0..40 {
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
                assert_eq!(player, P0);
                // Only basic land cards should be offered — never Badlands or Grizzly Bears.
                assert!(!options.is_empty());
                assert!(
                    !options.contains(&nonbasic),
                    "nonbasic land must not be offered for a basic land search: {options:?}"
                );
                for &id in &options {
                    assert!(
                        id == basic_forest || id == basic_island,
                        "only basic land cards should be offered: {id:?}"
                    );
                }
                searches += 1;
                engine
                    .apply(
                        P0,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected pending during Sprouting Vines: {other:?}"),
        }
    }
    assert_eq!(searches, 2, "original + 1 Storm copy each searched once");
    assert_eq!(engine.state().per_turn.spells_cast, vec![2, 0]);
    assert!(in_hand(&engine, P0, forest()).is_some());
    assert!(in_hand(&engine, P0, island()).is_some());
    assert!(in_hand(&engine, P0, badlands()).is_none());
}
