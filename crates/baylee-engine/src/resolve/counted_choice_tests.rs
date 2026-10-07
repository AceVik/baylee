use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};

fn me() -> PlayerId {
    PlayerId::new(0)
}

/// A two-seat game whose seat 0 has `cards` cards in its library.
fn library_of(cards: usize) -> (GameState, Vec<ObjectId>) {
    let mut state = GameState::from_preset(&preset(311, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game");
    let library = state.zones.list(ZoneLocation::Library(me())).clone();
    for card in library {
        let _ = state.move_object(
            card,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            Cause::Effect,
        );
    }
    let name = state.names.intern("Card");
    let made = (0..cards)
        .map(|_| state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Library(me())))
        .collect();
    (state, made)
}

fn mine(effect: Effect) -> Resolution {
    Resolution {
        source: ObjectId::NO_SOURCE,
        on_stack: ObjectId::NO_SOURCE,
        controller: me(),
        effects: vec![effect],
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        event_mana: None,
        retarget_left: None,
    }
}

/// "Put two of them into your hand" over a library of one asks for the
/// one (CR 609.3). It asked for two, and nothing could answer.
#[test]
fn a_look_that_picks_more_than_the_library_holds_asks_for_what_is_there() {
    let (mut state, cards) = library_of(1);
    let mut res = mine(Effect::LookAtTopPick {
        count: baylee_cards_dsl::Amount::Fixed(7),
        pick: 2,
        random: false,
    });
    let Flow::Wait(Pending::ChooseCards {
        options, min, max, ..
    }) = run(&mut state, &mut res)
    else {
        panic!("the look asks");
    };
    assert_eq!(options, cards);
    assert_eq!((min, max), (1, 1));
}

/// A search told to find two cards over a library holding one finds as
/// many as possible (CR 701.23d): one.
#[test]
fn a_search_for_more_than_the_library_holds_finds_what_is_there() {
    const TWO: &[baylee_cards_dsl::effect::Find] = &[
        baylee_cards_dsl::effect::Find::HAND,
        baylee_cards_dsl::effect::Find::HAND,
    ];
    let (mut state, cards) = library_of(1);
    let mut res = mine(Effect::SearchLibrary {
        filter: &baylee_cards_dsl::Filter::Any,
        finds: TWO,
        optional: false,
    });
    let Flow::Wait(Pending::ChooseCards {
        options, min, max, ..
    }) = run(&mut state, &mut res)
    else {
        panic!("the search asks");
    };
    assert_eq!(options, cards);
    assert_eq!((min, max), (1, 1));
}
