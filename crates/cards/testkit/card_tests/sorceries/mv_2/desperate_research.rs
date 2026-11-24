//! `cards/sorceries/mv_2/desperate_research.rs`, played.
//!
//! Oracle:
//! - "Choose a card name other than a basic land card name. Reveal the top seven cards of your library and put all of them with that name into your hand. Exile the rest."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::{legends_kit as lk, partials_w1b_kit as kit};
use baylee_core::{generated::index, types::SupertypeSet};

const P0: PlayerId = PlayerId::new(0);

fn desperate_research() -> CardIndex {
    card_index("308b49ad-ebab-41c4-9e09-44202549bafc")
}

struct Cast {
    engine: Engine<RegistryLookup>,
    stacked: Vec<ObjectId>,
    spell: ObjectId,
}

/// Casts Research with exactly `top_first` in the library and stops at its
/// name choice.
fn research(top_first: &[CardIndex]) -> Cast {
    let research = desperate_research();
    let mut hand = vec![research];
    hand.extend_from_slice(top_first);
    let mut engine = Duel::new(SEED, index::SWAMP).hand(0, &hand).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let spell = in_hand(&engine, P0, research).expect("Research starts in hand");
    let stacked = kit::replace_library(&mut engine, P0, top_first);
    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut engine, P0, research);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    Cast {
        engine,
        stacked,
        spell,
    }
}

fn choose(mut cast: Cast, named: CardIndex) -> Cast {
    cast.engine
        .apply(
            P0,
            PlayerAction::ChooseCardName {
                card: named,
                face: 0,
            },
        )
        .expect("the selected nonbasic card name is legal");
    pass_until(&mut cast.engine, stack_is_empty);
    cast
}

fn zone(engine: &Engine<RegistryLookup>, zone: ZoneLocation) -> Vec<ObjectId> {
    engine.state().zones.list(zone).clone()
}

fn reveals(engine: &Engine<RegistryLookup>) -> Vec<(u64, Vec<ObjectId>)> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|entry| match &entry.event {
            GameEvent::Revealed { cards, .. } => Some((entry.seq, cards.clone())),
            _ => None,
        })
        .collect()
}

#[track_caller]
fn assert_only_research_in_graveyard(cast: &Cast) {
    assert_eq!(kit::graveyard_of(&cast.engine, P0), vec![cast.spell]);
}

#[track_caller]
fn assert_no_draw(engine: &Engine<RegistryLookup>) {
    assert!(
        !engine
            .state()
            .journal
            .entries()
            .iter()
            .any(|entry| matches!(entry.event, GameEvent::CardsDrawn { .. })),
        "putting revealed cards into hand is not drawing them"
    );
}

#[test]
fn desperate_research_refuses_basic_names_but_allows_a_nonbasic_land() {
    let mut cast = research(&[index::WASTELAND, index::SWAMP]);
    assert!(matches!(
        cast.engine.pending(),
        Pending::ChooseCardName {
            excluded_supertypes: SupertypeSet::BASIC,
            ..
        }
    ));
    for basic in [index::FOREST, index::PLAINS] {
        assert!(
            cast.engine
                .apply(
                    P0,
                    PlayerAction::ChooseCardName {
                        card: basic,
                        face: 0,
                    },
                )
                .is_err(),
            "a basic land card name is outside the choice"
        );
        assert!(matches!(
            cast.engine.pending(),
            Pending::ChooseCardName {
                excluded_supertypes: SupertypeSet::BASIC,
                ..
            }
        ));
    }

    let cast = choose(cast, index::WASTELAND);
    assert_eq!(
        zone(&cast.engine, ZoneLocation::Hand(P0)),
        vec![cast.stacked[0]],
        "Wasteland is a legal nonbasic land name"
    );
    assert_eq!(
        zone(&cast.engine, ZoneLocation::Exile(P0)),
        vec![cast.stacked[1]]
    );
    assert_only_research_in_graveyard(&cast);
    assert_no_draw(&cast.engine);
}

#[test]
fn desperate_research_reveals_exactly_seven_once_then_sorts_all_matches() {
    let cast = choose(
        research(&[
            index::LIGHTNING_BOLT,
            index::SWAMP,
            index::FOREST,
            index::LIGHTNING_BOLT,
            index::MOUNTAIN,
            index::PLAINS,
            index::LIGHTNING_BOLT,
            index::ISLAND,
            index::LIGHTNING_BOLT,
            index::SWAMP,
        ]),
        index::LIGHTNING_BOLT,
    );

    let revealed = reveals(&cast.engine);
    assert_eq!(revealed.len(), 1, "all seven share one reveal event");
    let (reveal_seq, cards) = &revealed[0];
    assert_eq!(cards, &cast.stacked[..7], "top-first reveal order");

    let moves: Vec<_> = cast
        .engine
        .state()
        .journal
        .entries()
        .iter()
        .filter(|entry| {
            entry.seq > *reveal_seq
                && matches!(
                    &entry.event,
                    GameEvent::ZoneChanged { object, .. }
                        if cast.stacked[..7].contains(object)
                )
        })
        .collect();
    assert_eq!(
        moves.len(),
        7,
        "each revealed card moves exactly once after the reveal: {moves:#?}"
    );

    assert_eq!(
        zone(&cast.engine, ZoneLocation::Hand(P0)),
        vec![cast.stacked[0], cast.stacked[3], cast.stacked[6]],
        "every match among the seven goes to hand in top-first order"
    );
    assert_eq!(
        zone(&cast.engine, ZoneLocation::Exile(P0)),
        vec![
            cast.stacked[1],
            cast.stacked[2],
            cast.stacked[4],
            cast.stacked[5],
        ],
        "every nonmatch goes to exile"
    );
    assert_eq!(
        kit::library_of(&cast.engine, P0),
        vec![cast.stacked[9], cast.stacked[8], cast.stacked[7]],
        "the suffix, including another match, is neither revealed nor moved"
    );
    assert_only_research_in_graveyard(&cast);
    assert_no_draw(&cast.engine);
}

#[test]
fn desperate_research_uses_every_card_in_a_short_or_empty_library() {
    for size in [0, 3] {
        let top_first = if size == 0 {
            vec![]
        } else {
            vec![index::LIGHTNING_BOLT, index::SWAMP, index::LIGHTNING_BOLT]
        };
        let cast = choose(research(&top_first), index::LIGHTNING_BOLT);
        let expected_hand: Vec<_> = top_first
            .iter()
            .zip(&cast.stacked)
            .filter_map(|(card, object)| (*card == index::LIGHTNING_BOLT).then_some(*object))
            .collect();
        let expected_exile: Vec<_> = top_first
            .iter()
            .zip(&cast.stacked)
            .filter_map(|(card, object)| (*card != index::LIGHTNING_BOLT).then_some(*object))
            .collect();

        assert_eq!(zone(&cast.engine, ZoneLocation::Hand(P0)), expected_hand);
        assert_eq!(zone(&cast.engine, ZoneLocation::Exile(P0)), expected_exile);
        assert!(kit::library_of(&cast.engine, P0).is_empty());
        let revealed = reveals(&cast.engine);
        if cast.stacked.is_empty() {
            assert!(revealed.is_empty(), "an empty library reveals nothing");
        } else {
            assert_eq!(revealed, vec![(revealed[0].0, cast.stacked.clone())]);
        }
        assert_only_research_in_graveyard(&cast);
        assert_no_draw(&cast.engine);
    }
}
