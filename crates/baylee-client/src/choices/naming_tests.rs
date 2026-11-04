//! Naming a card: the rows the deck builder's search leaves, the question's
//! own restriction beside it, and what a row answers.

use super::*;
use baylee_core::ids::PlayerId;
use baylee_core::types::TypeSet;

const NOTHING: TypeSet = TypeSet::EMPTY;

fn labels(rows: &[ChoiceOption]) -> Vec<&str> {
    rows.iter().map(|row| row.label.as_str()).collect()
}

/// The pool as a client in `name`'s language is served it: Lightning Bolt by
/// that name, and a name in a third language filed where no search reads.
fn served(name: &str, foreign: &str) -> Vec<PoolCard> {
    let index = baylee_cards::decks::by_name("Lightning Bolt")
        .expect("in the pool")
        .get();
    let mut bolt = registry_pool()
        .get(&index)
        .expect("in the registry's rows")
        .clone();
    bolt.name = name.to_string();
    bolt.foreign_names = vec![foreign.to_string()];
    vec![bolt]
}

/// The deck builder's own language, so every criterion it has is here.
#[test]
fn the_search_is_the_deck_builders_with_every_criterion_it_has() {
    let rows = card_name_rows("t:land name:wasteland", NOTHING, NOTHING);
    assert_eq!(labels(&rows), ["Wasteland"]);
    let rows = card_name_rows("name:bolt mv=1 c:r", NOTHING, NOTHING);
    assert!(labels(&rows).contains(&"Lightning Bolt"), "{rows:?}");
    assert!(card_name_rows("name:bolt mv=9", NOTHING, NOTHING).is_empty());
}

/// A row answers what it says: the search moves the rows, and a row's index
/// stays its place in the one fixed order, through `pick`.
#[test]
fn a_row_answers_the_card_it_names_however_the_list_is_narrowed() {
    let bolt = baylee_cards::decks::by_name("Lightning Bolt").expect("in the pool");
    for search in ["lightning bolt", "name:bolt mv=1 c:r"] {
        let rows = card_name_rows(search, NOTHING, NOTHING);
        let row = rows
            .iter()
            .find(|row| row.label == "Lightning Bolt")
            .expect("the row");
        assert_eq!(card_name_at(row.index), Some((bolt, 0)));
        assert_eq!(row.card.map(|card| card.index), Some(bolt), "its preview");
    }
}

/// The question's restriction stands apart from the search: "nonland" holds
/// whatever is typed, and no filter can take it away.
#[test]
fn the_questions_restriction_holds_under_any_search() {
    let rows = card_name_rows("wasteland", NOTHING, TypeSet::LAND);
    assert!(!labels(&rows).contains(&"Wasteland"), "{rows:?}");
    // The question reads the face a row names: a land on one face of a card
    // leaves its other face a name this question takes.
    let rows = card_name_rows("t:land", NOTHING, TypeSet::LAND);
    assert!(
        rows.iter()
            .all(|row| !card_names()[row.index].types.intersects(TypeSet::LAND)),
        "{rows:?}"
    );
    assert!(!card_name_rows("t:land", NOTHING, NOTHING).is_empty());
    assert!(card_name_rows("sol ring", TypeSet::LAND, NOTHING).is_empty());
}

#[test]
fn the_restriction_is_said_in_words_and_none_is_said_when_there_is_none() {
    assert_eq!(name_restriction(NOTHING, NOTHING, Lang::En), None);
    assert_eq!(
        name_restriction(NOTHING, TypeSet::LAND, Lang::En).as_deref(),
        Some("This question leaves out: Land")
    );
    assert_eq!(
        name_restriction(TypeSet::LAND, NOTHING, Lang::De).as_deref(),
        Some("Diese Frage nimmt nur: Land")
    );
}

/// The rule of every search: the client's language and English, and no other.
/// A Spanish client finds the card as Rayo and as Lightning Bolt, and its
/// German name is a third language there.
#[test]
fn a_name_is_found_in_the_clients_language_and_english_and_no_other() {
    let pool = served("Rayo", "Blitzschlag");
    let search = |text| Search {
        text,
        pool: Some(&pool),
        facts: None,
    };
    let rows = card_name_rows(search("rayo"), NOTHING, NOTHING);
    let row = rows.first().expect("its own language finds it");
    assert_eq!(row.label, "Rayo (Lightning Bolt)", "both names are shown");
    assert!(!card_name_rows(search("lightning bolt"), NOTHING, NOTHING).is_empty());
    assert!(card_name_rows(search("blitzschlag"), NOTHING, NOTHING).is_empty());
    // A German client the other way about: Spanish is the third language.
    let pool = served("Blitzschlag", "Rayo");
    let search = |text| Search {
        text,
        pool: Some(&pool),
        facts: None,
    };
    assert!(!card_name_rows(search("blitzschlag"), NOTHING, NOTHING).is_empty());
    assert!(card_name_rows(search("rayo"), NOTHING, NOTHING).is_empty());
}

/// A controlled player's question reads and answers the same rows: a row is
/// a place in the pool, and the seat is the interaction's.
#[test]
fn a_controlled_players_question_is_answered_the_same_way() {
    let them = PlayerId::new(1);
    let pending = baylee_engine::choice::Pending::ChooseCardName {
        types: NOTHING,
        excluded_types: TypeSet::LAND,
        player: them,
    };
    let mut interaction = Interaction::new(pending, them);
    let rows = options(
        &interaction.prompt(),
        Lang::En,
        None,
        "lightning bolt",
        FaceNames::default(),
    )
    .expect("a chooser");
    let row = rows
        .iter()
        .find(|row| row.label == "Lightning Bolt")
        .expect("the row");
    assert!(pick(&mut interaction, row.index));
    assert!(matches!(
        interaction.confirm(),
        Some(baylee_engine::choice::PlayerAction::ChooseCardName { .. })
    ));
}

/// The walk is kept: the frames that ask again ask nothing of the pool.
#[test]
fn asking_again_answers_the_same_rows_and_a_new_search_new_ones() {
    let first = card_name_rows("name:needle", NOTHING, NOTHING);
    assert_eq!(first, card_name_rows("name:needle", NOTHING, NOTHING));
    assert_ne!(first, card_name_rows("name:wasteland", NOTHING, NOTHING));
}

/// The chooser is a pool list, so the table's keys reach it as they reach the
/// deck builder, and without the table it offers no card rather than a wrong
/// one.
#[test]
fn the_table_narrows_the_names_and_without_it_nothing_is_guessed() {
    use baylee_client_core::cardquery::{PoolFacts, PoolFactsBody, PoolSet};
    let index = baylee_cards::decks::by_name("Lightning Bolt")
        .expect("in the pool")
        .get();
    let facts: PoolFacts = PoolFactsBody::assemble(
        vec![PoolSet {
            code: "m21".to_string(),
            name: "Core Set 2021".to_string(),
        }],
        vec![baylee_cardtext::CardLine {
            index,
            legalities: [("modern".to_string(), "legal".to_string())].into(),
            printed: vec![("m21".to_string(), "uncommon".to_string())],
        }],
    )
    .into();
    let pool = served("Rayo", "Blitzschlag");
    let informed = |text| Search {
        text,
        pool: Some(&pool),
        facts: Some(&facts),
    };
    assert!(!card_name_rows(informed("s:m21 rayo"), NOTHING, NOTHING).is_empty());
    assert!(!card_name_rows(informed("f:modern rayo"), NOTHING, NOTHING).is_empty());
    assert!(card_name_rows(informed("r:mythic rayo"), NOTHING, NOTHING).is_empty());
    let bare = |text| Search {
        text,
        pool: Some(&pool),
        facts: None,
    };
    for line in ["s:m21 rayo", "-s:m21 rayo", "f:modern rayo"] {
        assert!(
            card_name_rows(bare(line), NOTHING, NOTHING).is_empty(),
            "{line}"
        );
    }
}
