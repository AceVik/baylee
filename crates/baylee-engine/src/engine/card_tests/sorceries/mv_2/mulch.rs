//! `cards/sorceries/mv_2/mulch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn mulch() -> CardIndex {
    card_index("12c03a03-36bf-487e-b8be-e55399525da5")
}

/// Mulch (`12c03a03-36bf-487e-b8be-e55399525da5`) — {1}{G} Sorcery (Implemented):
/// "Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard."
#[test]
fn mulch_puts_revealed_lands_into_hand_and_rest_into_graveyard() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[mulch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();
    cast_from_hand(&mut engine, P0, mulch());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before - 1 + 4,
        "Mulch should put all four revealed Forests from the top of the library into P0's hand"
    );
}

#[test]
fn nonlands_go_to_graveyard_and_only_four_cards_are_revealed() {
    use crate::event::Cause;
    use crate::zone::ZonePosition;
    use baylee_core::generated::index;
    let mut e = Duel::new(SEED, index::GRIZZLY_BEARS)
        .hand(0, &[mulch(), forest(), plains()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for c in [forest(), plains()] {
        let id = in_hand(&e, P0, c).unwrap();
        e.dev_state_mut(P0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Library(P0),
                ZonePosition::Top,
                Cause::Setup,
            )
            .unwrap();
    }
    e.refresh_offer();
    let before = e.state().zones.list(ZoneLocation::Library(P0)).len();
    lk::float(&mut e, P0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, P0, mulch());
    lk::settle(&mut e);
    assert!(in_hand(&e, P0, forest()).is_some());
    assert!(in_hand(&e, P0, plains()).is_some());
    assert!(in_hand(&e, P0, index::GRIZZLY_BEARS).is_none());
    let creatures = e
        .state()
        .zones
        .list(ZoneLocation::Graveyard(P0))
        .iter()
        .filter(|id| {
            e.state()
                .object(**id)
                .unwrap()
                .card
                .is_some_and(|c| c.index == index::GRIZZLY_BEARS)
        })
        .count();
    assert_eq!(creatures, 2);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        before - 4
    );
}
