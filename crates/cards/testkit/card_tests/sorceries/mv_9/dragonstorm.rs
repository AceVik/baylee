//! `cards/sorceries/mv_9/dragonstorm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
use baylee_engine::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);

/// Dragonstorm — {8}{R} Sorcery:
/// "Search your library for a Dragon permanent card, put it onto the battlefield, then shuffle.
/// Storm (When you cast this spell, copy it for each spell cast before it this turn.)"
#[test]
fn dragonstorm_puts_one_dragon_permanent_onto_battlefield_per_storm_copy_plus_original() {
    let dragonstorm = card_index("d74c3871-018a-4450-a613-f67703bb0d72");
    let mut engine = Duel::new(SEED, index::GRIZZLY_BEARS)
        .hand(
            0,
            &[
                index::DARK_RITUAL,
                dragonstorm,
                index::SHIVAN_DRAGON,
                index::SHIVAN_DRAGON,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let d1 = hand_to_library_top(&mut engine, P0, index::SHIVAN_DRAGON);
    let d2 = hand_to_library_top(&mut engine, P0, index::SHIVAN_DRAGON);
    let shuffles_before = engine
        .journal()
        .entries()
        .iter()
        .filter(|e| {
            matches!(
                e.event,
                GameEvent::Shuffled {
                    player,
                    zone: Zone::Library
                } if player == P0
            )
        })
        .count();

    {
        let state = engine.dev_state_mut(P0).unwrap();
        state.players[0].mana_pool.add(ManaColor::Black, 1);
        state.players[0].mana_pool.add(ManaColor::Red, 9);
    }
    engine.refresh_offer();

    // 1 prior spell -> Storm creates 1 copy -> 2 Dragon searches onto battlefield.
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, dragonstorm);
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
                assert!(!options.is_empty());
                for &id in &options {
                    assert!(
                        id == d1 || id == d2,
                        "only Dragon permanent cards should be offered: {id:?}"
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
            other => panic!("unexpected pending during Dragonstorm: {other:?}"),
        }
    }

    assert_eq!(searches, 2, "original + 1 Storm copy each searched once");
    assert_eq!(engine.state().per_turn.spells_cast, vec![2, 0]);
    assert_eq!(
        engine.state().object(d1).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    assert_eq!(
        engine.state().object(d2).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
    let shuffles_after = engine
        .journal()
        .entries()
        .iter()
        .filter(|e| {
            matches!(
                e.event,
                GameEvent::Shuffled {
                    player,
                    zone: Zone::Library
                } if player == P0
            )
        })
        .count();
    assert_eq!(shuffles_after - shuffles_before, 2);
}
