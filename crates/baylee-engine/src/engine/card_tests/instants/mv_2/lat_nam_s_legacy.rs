//! Lat-Nam's Legacy — `{1}{U}` Instant (Alliances)
//!
//! (`Coverage::Partial`: uses `BottomCardFromHand` followed by `ShuffleLibrary`;
//! if you choose a card from your hand, shuffles it into your library and draws
//! two cards at the beginning of the next turn's upkeep.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn lat_nam_s_legacy() -> CardIndex {
    card_index("f885ee54-3b26-4d61-9783-3930b0e625aa")
}

#[test]
fn lat_nam_s_legacy_shuffles_card_from_hand_into_library_and_draws_two_next_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[lat_nam_s_legacy(), grizzly_bears()])
        .battlefield(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let legacy = in_hand(&engine, p0, lat_nam_s_legacy()).unwrap();
    let bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let lib_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: legacy })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, grizzly_bears()).is_none());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        lib_before + 1
    );

    // Advance to next turn's upkeep (p1's turn) -> p0 draws 2 cards.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2
    );
}

/// Casts Lat-Nam's Legacy for player 0 off two Islands and returns the engine with the question
/// "shuffle which card" pending (hand: Legacy plus `others`).
fn legacy_asks_for_a_card(others: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut hand = vec![lat_nam_s_legacy()];
    hand.extend_from_slice(others);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &hand)
        .battlefield(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, lat_nam_s_legacy());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// "Shuffle a card from your hand into your library": any card of the hand is offered, the one
/// chosen leaves and the other stays; "draw two cards at the beginning of the next turn's upkeep"
/// happens once, in the opponent's upkeep, and the turn after brings only the ordinary draw.
#[test]
fn lat_nam_s_legacy_takes_the_chosen_card_and_draws_two_once() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = legacy_asks_for_a_card(&[grizzly_bears(), gray_ogre()]);
    let bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let ogre = in_hand(&engine, p0, gray_ogre()).unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the card question")
    };
    assert!(
        options.contains(&bears) && options.contains(&ogre),
        "{options:?}"
    );
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).len();

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ogre],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, gray_ogre()).is_none());
    assert!(
        in_hand(&engine, p0, grizzly_bears()).is_some(),
        "the other card stays"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library + 1
    );
    assert_eq!(hand_size(&engine, p0), 1);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        hand_size(&engine, p0),
        3,
        "two cards in the opponent's upkeep"
    );
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(hand_size(&engine, p0), 4, "and then only the ordinary draw");
}

/// "If you do": with nothing else in hand there is no card to shuffle, so no question is asked and
/// no cards are drawn next upkeep (the pass walk refuses any question on the way).
#[test]
fn lat_nam_s_legacy_with_an_empty_hand_draws_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[lat_nam_s_legacy()])
        .battlefield(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, lat_nam_s_legacy());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_size(&engine, p0), 0);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        hand_size(&engine, p0),
        0,
        "nothing was shuffled, so nothing is drawn"
    );
}

/// Known gap, pinned: the printed text makes the player shuffle a card, but this card lets them
/// name none, and then it does nothing (less than printed, never more). When the DSL can ask for
/// exactly one card, this test must move to the side where declining is refused.
#[test]
fn lat_nam_s_legacy_lets_the_player_decline_and_then_does_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = legacy_asks_for_a_card(&[grizzly_bears()]);
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_size(&engine, p0), 1);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(hand_size(&engine, p0), 1, "no card shuffled, none drawn");
}
