//! `cards/sorceries/mv_5/void.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn void() -> CardIndex {
    card_index("fdb28fd3-70fb-43c3-b367-9cf0d2c46e06")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hand_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Hand(seat)).clone()
}

/// Void uses one resolution-scoped number for both halves. At one, both
/// players' Sol Rings and Llanowar Elves die, while their two-mana Bears and
/// the zero-mana artifact creature survive. Only the targeted hand is
/// revealed: its one-mana nonlands are discarded, its two-mana creature and
/// land stay, and the caster's otherwise matching hand cards are untouched.
#[test]
fn void_uses_the_chosen_number_on_battlefield_and_only_in_the_target_hand() {
    let mut engine = lk::game(
        SEED,
        &[sol_ring(), llanowar_elves(), ornithopter(), grizzly_bears()],
        &[sol_ring(), llanowar_elves(), grizzly_bears()],
        &[void(), lightning_bolt(), forest()],
        &[
            forest(),
            lightning_bolt(),
            sol_ring(),
            grizzly_bears(),
            dark_ritual(),
        ],
    );
    let target_hand = hand_of(&engine, P1);
    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 3),
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
        ],
    );

    cast_with_floating(&mut engine, P0, void());
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!(
            "Void asks for its target player, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, P0);
    assert_eq!(options, vec![P0, P1], "Void can target either player");
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    let Pending::ChooseNumber {
        player,
        min,
        max,
        reason,
    } = engine.pending().clone()
    else {
        unreachable!("just checked")
    };
    assert_eq!(player, P0, "Void's controller chooses the number");
    assert_eq!((min, max), (0, u32::MAX));
    assert_eq!(reason, baylee_engine::choice::NumberPrompt::Choice);
    engine.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    for seat in [P0, P1] {
        assert!(on_battlefield(&engine, seat, sol_ring()).is_none());
        assert!(on_battlefield(&engine, seat, llanowar_elves()).is_none());
        assert!(in_graveyard(&engine, seat, sol_ring()).is_some());
        assert!(in_graveyard(&engine, seat, llanowar_elves()).is_some());
        assert!(
            on_battlefield(&engine, seat, grizzly_bears()).is_some(),
            "a creature with another mana value survives for {seat:?}"
        );
    }
    assert!(
        on_battlefield(&engine, P0, ornithopter()).is_some(),
        "an artifact creature with another mana value survives"
    );

    assert_eq!(hand_of(&engine, P1).len(), 2);
    assert!(in_hand(&engine, P1, forest()).is_some(), "the land stays");
    assert!(
        in_hand(&engine, P1, grizzly_bears()).is_some(),
        "the nonland with another mana value stays"
    );
    for card in [lightning_bolt(), sol_ring(), dark_ritual()] {
        assert!(in_hand(&engine, P1, card).is_none());
        assert!(in_graveyard(&engine, P1, card).is_some());
    }
    assert_eq!(
        hand_of(&engine, P0).len(),
        2,
        "the caster's hand is not discarded"
    );
    assert!(in_hand(&engine, P0, lightning_bolt()).is_some());
    assert!(in_hand(&engine, P0, forest()).is_some());

    let reveals: Vec<_> = engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match &entry.event {
            GameEvent::Revealed { player, cards } => Some((*player, cards.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        reveals,
        vec![(P1, target_hand)],
        "exactly the targeted player's whole hand is revealed"
    );
}
