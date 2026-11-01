//! `Effect::RevealLibrarySearch`: search your library for a card of a stated
//! quality and reveal it (Grim Reminder's "search your library for a nonland
//! card and reveal it"). "If a player is searching a hidden zone for cards
//! with a stated quality, such as a card with a certain card type or color,
//! that player isn't required to find some or all of those cards even if
//! they're present in that zone" (CR 701.23b).

use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_core::ids::SeatSet;

const ME: PlayerId = PlayerId::new(0);

/// Seat 0's library of Forests with one card that is not a land in it, and a
/// spell searching it for a card matching `filter`.
fn searching(filter: &'static baylee_cards_dsl::Filter) -> (GameState, Resolution, ObjectId) {
    let mut state =
        GameState::from_preset(&preset(31, &[]), &SyntheticLookup::new(vec![])).unwrap();
    let name = state.names.intern("A card that is not a land");
    let card = state.create_bare(ME, ObjectKind::Card, name, ZoneLocation::Library(ME));
    let name = state.names.intern("Searching spell");
    let source = state.create_bare(ME, ObjectKind::Spell, name, ZoneLocation::Stack);
    let res = Resolution {
        source,
        on_stack: source,
        controller: ME,
        effects: vec![Effect::RevealLibrarySearch {
            filter,
            optional: false,
        }],
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: SeatSet::new(),
        event_object: None,
        event_mana: None,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        this_way_from: None,
        coin_stake: None,
    };
    (state, res, card)
}

fn revealed(state: &GameState) -> usize {
    state
        .journal
        .entries()
        .iter()
        .filter(|e| matches!(e.event, GameEvent::Revealed { .. }))
        .count()
}

/// A nonland card is a stated quality: the searcher is offered the one
/// there is and may find nothing, and then nothing is revealed.
#[test]
fn a_search_for_a_stated_quality_may_find_nothing() {
    let (mut state, mut res, card) = searching(&baylee_cards_dsl::Filter::NONLAND);
    let Flow::Wait(Pending::ChooseCards { options, min, .. }) = run(&mut state, &mut res) else {
        panic!("the searcher is asked");
    };
    assert_eq!(options, vec![card]);
    assert_eq!(min, 0, "CR 701.23b: not required to find it");
    let before = revealed(&state);
    assert!(matches!(resume(&mut state, &mut res, &[]), Flow::Complete));
    assert_eq!(revealed(&state), before, "nothing found, nothing revealed");
}

/// The one search held to finding a card: for any card at all, which names
/// no quality to fail to find.
#[test]
fn a_search_for_any_card_must_find_one() {
    let (mut state, mut res, _) = searching(&baylee_cards_dsl::Filter::Any);
    let Flow::Wait(Pending::ChooseCards { min, .. }) = run(&mut state, &mut res) else {
        panic!("the searcher is asked");
    };
    assert_eq!(min, 1);
}
