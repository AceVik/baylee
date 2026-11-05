//! `cards/creatures/mv_4/goblin_ringleader.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

fn ringleader() -> CardIndex {
    card_index("4100e486-0d27-436c-8429-76bc2c1a26ab")
}

fn put_from_hand_on_top(e: &mut Engine<RegistryLookup>, card: CardIndex) -> ObjectId {
    hand_to_library_top(e, lk::P0, card)
}

#[test]
fn goblin_ringleader_takes_every_goblin_orders_the_rest_and_attacks_at_once() {
    let mut e = lk::game(
        SEED,
        &[],
        &[],
        &[
            ringleader(),
            index::RAGING_GOBLIN,
            index::GOBLIN_SKY_RAIDER,
            index::SHOCK,
            grizzly_bears(),
        ],
        &[],
    );
    let bears = put_from_hand_on_top(&mut e, grizzly_bears());
    let shock = put_from_hand_on_top(&mut e, index::SHOCK);
    put_from_hand_on_top(&mut e, index::GOBLIN_SKY_RAIDER);
    put_from_hand_on_top(&mut e, index::RAGING_GOBLIN);

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut e, lk::P0, ringleader());
    pass_until(&mut e, |e| matches!(e.pending(), Pending::Arrange { .. }));
    let Pending::Arrange { cards, .. } = e.pending().clone() else {
        panic!("only the two misses need ordering: {:?}", e.pending());
    };
    assert_eq!(cards, vec![shock, bears]);
    e.apply(
        lk::P0,
        PlayerAction::Arrange {
            piles: vec![vec![shock, bears]],
        },
    )
    .unwrap();
    lk::settle(&mut e);
    assert!(in_hand(&e, lk::P0, index::RAGING_GOBLIN).is_some());
    assert!(in_hand(&e, lk::P0, index::GOBLIN_SKY_RAIDER).is_some());
    assert!(in_hand(&e, lk::P0, index::SHOCK).is_none());
    let source = lk::obj(&e, lk::P0, ringleader());
    assert!(lk::attackers_offered(&mut e).contains(&source));
}

fn exile_all_but_top_two(e: &mut Engine<RegistryLookup>) {
    let library = e.state().zones.list(ZoneLocation::Library(lk::P0)).clone();
    for card in library.into_iter().take(library_size(e, lk::P0) - 2) {
        e.dev_state_mut(lk::P0)
            .unwrap()
            .move_object(
                card,
                ZoneLocation::Exile(lk::P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
}

#[test]
fn goblin_ringleader_reveals_only_the_cards_available() {
    let mut e = lk::game(
        SEED,
        &[],
        &[],
        &[ringleader(), index::RAGING_GOBLIN, index::SHOCK],
        &[],
    );
    hand_to_library_top(&mut e, lk::P0, index::SHOCK);
    hand_to_library_top(&mut e, lk::P0, index::RAGING_GOBLIN);
    exile_all_but_top_two(&mut e);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut e, lk::P0, ringleader());
    lk::settle(&mut e);
    assert!(in_hand(&e, lk::P0, index::RAGING_GOBLIN).is_some());
    assert_eq!(library_size(&e, lk::P0), 1);
    assert!(!matches!(e.pending(), Pending::ChooseCards { .. }));
}

#[test]
fn goblin_ringleader_with_an_empty_library_does_not_ask_a_question() {
    let mut e = lk::game(SEED, &[], &[], &[ringleader()], &[]);
    let library = e.state().zones.list(ZoneLocation::Library(lk::P0)).clone();
    for card in library {
        e.dev_state_mut(lk::P0)
            .unwrap()
            .move_object(
                card,
                ZoneLocation::Exile(lk::P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut e, lk::P0, ringleader());
    lk::settle(&mut e);
    assert!(matches!(e.pending(), Pending::Priority { .. }));
    assert_eq!(library_size(&e, lk::P0), 0);
}
