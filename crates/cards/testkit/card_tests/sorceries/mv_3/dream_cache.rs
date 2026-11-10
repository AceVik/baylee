//! `cards/sorceries/mv_3/dream_cache.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn dream_cache() -> CardIndex {
    card_index("81338d64-8fc3-41ac-afc8-e2a79df1159b")
}

/// Dream Cache (81338d64-8fc3-41ac-afc8-e2a79df1159b) — {2}{U} Sorcery:
/// "Draw three cards, then put two cards from your hand both on top or both on
/// the bottom of your library."
#[test]
fn dream_cache_draws_three_cards_and_returns_two_from_hand_to_library() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[dream_cache()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, dream_cache());

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

/// "put two cards from your hand ... on top of your library": the two cards
/// chosen are the top two cards of the library afterwards, in the library and
/// not in the graveyard or hand.
#[test]
fn dream_cache_puts_the_two_chosen_cards_on_top_of_the_library() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[dream_cache()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, dream_cache());

    let mut chosen = vec![];
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
                chosen = vec![options[0], options[1]];
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: chosen.clone(),
                        },
                    )
                    .unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }

    assert_eq!(chosen.len(), 2);
    let library = engine.state().zones.list(ZoneLocation::Library(P0)).clone();
    let top_two = &library[library.len() - 2..];
    for card in &chosen {
        assert!(
            top_two.contains(card),
            "{card:?} is not on top: {top_two:?}"
        );
    }
    assert!(in_graveyard(&engine, P0, dream_cache()).is_some());
}
