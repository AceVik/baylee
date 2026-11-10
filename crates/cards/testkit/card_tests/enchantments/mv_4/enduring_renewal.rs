//! `cards/enchantments/mv_4/enduring_renewal.rs`, played.
//!
//! Oracle:
//! - "Play with your hand revealed."
//! - "If you would draw a card, reveal the top card of your library instead. If it's a creature card, put it into your graveyard. Otherwise, draw a card."
//! - "Whenever a creature is put into your graveyard from the battlefield, return it to your hand."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "47a080c4-ff04-4f52-aca0-2b8e4f4d931e"
fn enduring_renewal() -> CardIndex {
    card_index("47a080c4-ff04-4f52-aca0-2b8e4f4d931e")
}

#[test]
fn enduring_renewal_reveals_only_its_controllers_hand() {
    let mut engine = lk::game(SEED, &[], &[], &[enduring_renewal()], &[]);
    assert!(!engine.state().hand_revealed(P0));
    assert!(!engine.state().hand_revealed(P1));

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
    );
    cast_with_floating(&mut engine, P0, enduring_renewal());
    pass_until(&mut engine, stack_is_empty);
    assert!(engine.state().hand_revealed(P0));
    assert!(!engine.state().hand_revealed(P1));

    let renewal = on_battlefield(&engine, P0, enduring_renewal()).expect("Enduring Renewal");
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            renewal,
            ZoneLocation::Graveyard(P0),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Setup,
        )
        .expect("remove Enduring Renewal");
    assert!(
        !engine.state().hand_revealed(P0),
        "the reveal lasts only while Enduring Renewal remains on the battlefield"
    );
}

#[test]
fn enduring_renewal_reveals_each_draw_and_only_mills_creatures() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[
            enduring_renewal(),
            index::DIVINATION,
            index::FOREST,
            index::GRIZZLY_BEARS,
        ],
        &[],
    );
    let bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).expect("Bears in hand");
    let forest = in_hand(&engine, P0, index::FOREST).expect("Forest in hand");
    for card in [bears, forest] {
        engine
            .dev_state_mut(P0)
            .unwrap()
            .move_object(
                card,
                ZoneLocation::Library(P0),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Setup,
            )
            .expect("stack the library");
    }
    engine.refresh_offer();

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
    );
    cast_with_floating(&mut engine, P0, enduring_renewal());
    pass_until(&mut engine, stack_is_empty);
    let draws_before = engine.state().per_turn.draws[P0.get() as usize];

    // The Forest is above the Bears: Divination draws the Forest, but its
    // second draw is replaced by revealing and milling the creature card.
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
    );
    cast_with_floating(&mut engine, P0, index::DIVINATION);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P0, index::FOREST).is_some());
    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(
        in_hand(&engine, P0, index::GRIZZLY_BEARS).is_none(),
        "a creature milled from the library did not die on the battlefield"
    );
    assert_eq!(
        engine.state().per_turn.draws[P0.get() as usize],
        draws_before + 1,
        "the creature replacement does not count as drawing a card"
    );
    for card in [forest, bears] {
        assert!(
            engine
                .state()
                .journal
                .entries()
                .iter()
                .any(|entry| matches!(
                    &entry.event,
                    GameEvent::Revealed { player, cards }
                        if *player == P0 && cards.contains(&card)
                )),
            "the would-be draw was publicly revealed before changing zones"
        );
    }
}

#[test]
fn enduring_renewal_returns_dying_creature_from_your_graveyard_to_your_hand() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[],
        &[enduring_renewal(), index::LIGHTNING_BOLT],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
    );
    cast_with_floating(&mut engine, P0, enduring_renewal());
    pass_until(&mut engine, stack_is_empty);

    // Kill Grizzly Bears with Lightning Bolt: Enduring Renewal triggers and returns it to P0's hand.
    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "creature put into your graveyard from the battlefield is returned to your hand"
    );
    assert!(
        in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_none(),
        "creature is no longer in the graveyard"
    );
}

#[test]
fn enduring_renewal_leaves_an_opponents_creature_in_their_graveyard() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS],
        &[enduring_renewal(), index::LIGHTNING_BOLT],
        &[],
    );
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
    );
    cast_with_floating(&mut engine, P0, enduring_renewal());
    pass_until(&mut engine, stack_is_empty);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    lk::aim(&mut engine, &[their_bears], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "a creature put into another player's graveyard is not returned"
    );
    assert!(in_hand(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_none());
}

/// The draw step is a draw: with a creature on top, the controller's draw is
/// replaced and the creature is milled, while the opponent's draw is not.
#[test]
fn enduring_renewal_replaces_only_its_controllers_draw_step_draw() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[],
        &[enduring_renewal(), index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
    );
    let mine = in_hand(&engine, P0, index::GRIZZLY_BEARS).expect("Bears in hand");
    let theirs = in_hand(&engine, P1, index::HILL_GIANT).expect("Giant in hand");
    for (seat, card) in [(P0, mine), (P1, theirs)] {
        engine
            .dev_state_mut(seat)
            .unwrap()
            .move_object(
                card,
                ZoneLocation::Library(seat),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Setup,
            )
            .expect("stack the library");
    }
    engine.refresh_offer();
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
    );
    cast_with_floating(&mut engine, P0, enduring_renewal());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, P1);
    assert!(
        in_hand(&engine, P1, index::HILL_GIANT).is_some(),
        "the opponent's draw is not replaced"
    );

    reach_their_main_phase(&mut engine, P0);
    assert!(
        in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "the draw step's draw is revealed and the creature milled"
    );
    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_none());
}
