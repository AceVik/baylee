//! `cards/instants/mv_1/demonic_consultation.rs`, played.
//!
//! Oracle:
//! - "Choose a card name. Exile the top six cards of your library, then reveal cards from the top of your library until you reveal a card with the chosen name. Put that card into your hand and exile all other cards revealed this way."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::{legends_kit as lk, partials_w1b_kit as kit};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn demonic_consultation() -> CardIndex {
    card_index("9a1412db-45ad-46ea-8f12-a85d203113d8")
}

struct Outcome {
    engine: Engine<RegistryLookup>,
    stacked: Vec<ObjectId>,
    consultation: ObjectId,
    library_before: Vec<ObjectId>,
}

/// Casts Consultation naming `named`, with a library containing exactly
/// `top_first` in that order, and resolves it through the name choice.
fn consult(top_first: &[CardIndex], named: CardIndex) -> Outcome {
    let consultation_card = demonic_consultation();
    let mut hand = vec![consultation_card];
    hand.extend_from_slice(top_first);
    let mut engine = Duel::new(SEED, index::SWAMP).hand(0, &hand).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let consultation =
        in_hand(&engine, P0, consultation_card).expect("Consultation starts in hand");
    let stacked = kit::replace_library(&mut engine, P0, top_first);
    let library_before = kit::library_of(&engine, P0);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut engine, P0, consultation_card);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseCardName {
                card: named,
                face: 0,
            },
        )
        .expect("any card name may be chosen");
    pass_until(&mut engine, stack_is_empty);
    Outcome {
        engine,
        stacked,
        consultation,
        library_before,
    }
}

fn zone(engine: &Engine<RegistryLookup>, zone: ZoneLocation) -> Vec<ObjectId> {
    engine.state().zones.list(zone).clone()
}

fn revealed(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match &entry.event {
            GameEvent::Revealed { cards, .. } => Some(cards.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

#[track_caller]
fn assert_only_consultation_went_to_the_graveyard(outcome: &Outcome) {
    assert_eq!(
        kit::graveyard_of(&outcome.engine, P0),
        vec![outcome.consultation],
        "the spell resolves to the graveyard; none of the library cards may join it"
    );
}

#[test]
fn demonic_consultation_exiles_six_before_finding_the_next_named_card() {
    let outcome = consult(
        &[
            index::LIGHTNING_BOLT,
            index::SWAMP,
            index::SWAMP,
            index::SWAMP,
            index::SWAMP,
            index::SWAMP,
            index::LIGHTNING_BOLT,
            index::FOREST,
        ],
        index::LIGHTNING_BOLT,
    );

    assert_eq!(
        zone(&outcome.engine, ZoneLocation::Exile(P0)),
        outcome.stacked[..6],
        "a named card among the initial six stays exiled and does not stop the reveal"
    );
    assert_eq!(
        zone(&outcome.engine, ZoneLocation::Hand(P0)),
        vec![outcome.stacked[6]],
        "the first named card below those six is put into hand, not drawn"
    );
    assert_eq!(
        revealed(&outcome.engine),
        vec![outcome.stacked[6]],
        "the initial six are exiled without being revealed"
    );
    let library = kit::library_of(&outcome.engine, P0);
    assert_eq!(library.len(), outcome.library_before.len() - 7);
    assert_eq!(library.last(), outcome.stacked.get(7));
    assert_only_consultation_went_to_the_graveyard(&outcome);
}

#[test]
fn demonic_consultation_exiles_every_revealed_miss_and_preserves_the_suffix() {
    let outcome = consult(
        &[
            index::SWAMP,
            index::SWAMP,
            index::SWAMP,
            index::SWAMP,
            index::SWAMP,
            index::SWAMP,
            index::FOREST,
            index::MOUNTAIN,
            index::LIGHTNING_BOLT,
            index::PLAINS,
        ],
        index::LIGHTNING_BOLT,
    );

    assert_eq!(
        zone(&outcome.engine, ZoneLocation::Exile(P0)),
        outcome.stacked[..8],
        "the initial six and both revealed misses are exiled"
    );
    assert_eq!(
        zone(&outcome.engine, ZoneLocation::Hand(P0)),
        vec![outcome.stacked[8]]
    );
    assert_eq!(revealed(&outcome.engine), outcome.stacked[6..9]);
    let library = kit::library_of(&outcome.engine, P0);
    assert_eq!(library.len(), outcome.library_before.len() - 9);
    assert_eq!(
        library.last(),
        outcome.stacked.get(9),
        "the card below the match is never revealed or moved"
    );
    assert_only_consultation_went_to_the_graveyard(&outcome);
}

#[test]
fn demonic_consultation_without_a_match_exiles_the_whole_remaining_library() {
    let outcome = consult(&[index::SWAMP; 8], index::LIGHTNING_BOLT);

    assert!(kit::library_of(&outcome.engine, P0).is_empty());
    assert!(zone(&outcome.engine, ZoneLocation::Hand(P0)).is_empty());
    let top_first: Vec<_> = outcome.library_before.iter().rev().copied().collect();
    assert_eq!(zone(&outcome.engine, ZoneLocation::Exile(P0)), top_first);
    assert_eq!(
        revealed(&outcome.engine),
        top_first[6..],
        "with no match, every card after the initial six is revealed before exile"
    );
    assert_only_consultation_went_to_the_graveyard(&outcome);
}

#[test]
fn demonic_consultation_handles_libraries_with_at_most_six_cards() {
    for size in [0, 5, 6] {
        let outcome = consult(&vec![index::SWAMP; size], index::LIGHTNING_BOLT);

        assert!(kit::library_of(&outcome.engine, P0).is_empty());
        assert!(zone(&outcome.engine, ZoneLocation::Hand(P0)).is_empty());
        assert_eq!(
            zone(&outcome.engine, ZoneLocation::Exile(P0)).len(),
            outcome.library_before.len(),
            "only the cards actually available are exiled"
        );
        assert!(
            revealed(&outcome.engine).is_empty(),
            "a library consumed by the initial exile has nothing to reveal"
        );
        assert_only_consultation_went_to_the_graveyard(&outcome);
    }
}
