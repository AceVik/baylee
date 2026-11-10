//! `cards/sorceries/mv_4/ransack.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ransack() -> CardIndex {
    card_index("5b4a400b-435d-4140-984b-65f034c6a9fc")
}

/// Ransack (`5b4a400b-435d-4140-984b-65f034c6a9fc`) — {3}{U} Sorcery (Implemented):
/// "Look at the top five cards of target player's library; put any on bottom and the rest on top, in any order."
#[test]
fn ransack_looks_at_and_reorders_top_five_cards_of_target_player_library() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[ransack()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, ransack());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. }) || stack_is_empty(e)
    });
    let Pending::Arrange {
        player, mut cards, ..
    } = engine.pending().clone()
    else {
        panic!("expected Arrange pending, got {:?}", engine.pending());
    };
    assert_eq!(player, P0, "caster orders opponents library");
    assert_eq!(cards.len(), 5);
    cards.reverse();
    let expected_top = *cards.first().unwrap();
    let bottom = cards.split_off(3);
    let expected_bottom = *bottom.last().unwrap();
    engine
        .apply(
            P0,
            PlayerAction::Arrange {
                piles: vec![cards, bottom],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P1))
        .last()
        .unwrap();
    assert_eq!(top, expected_top);
    assert_eq!(
        *engine
            .state()
            .zones
            .list(ZoneLocation::Library(P1))
            .first()
            .unwrap(),
        expected_bottom
    );
}
