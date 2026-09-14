use super::*;
use baylee_engine::choice::{PlayerAction, StandingAnswer};

fn ondu_cleric() -> u32 {
    baylee_cards::by_oracle_id("f4232466-dd6a-49bf-be6c-95905c3ded17")
        .expect("the card pool has Ondu Cleric")
        .index
        .get()
}

/// A stored answer has to arrive as the exact handle the engine keeps its
/// automation under. The two ends of that are now in different processes,
/// so the test drives both: what the gateway sends, read by the code the
/// engine reads it with.
#[test]
fn a_stored_answer_becomes_the_handle_the_engine_uses() {
    let stored = vec![
        store::StandingAnswer {
            card: ondu_cleric(),
            ability: 0,
            yes: true,
        },
        store::StandingAnswer {
            card: ondu_cleric(),
            ability: baylee_core::ids::AbilityRef::ENTERS,
            yes: false,
        },
    ];
    let actions = baylee_engine_server::standing_answers(&standing_payload(&stored));
    assert_eq!(
        actions,
        vec![
            PlayerAction::SetStandingAnswer {
                ability: baylee_core::ids::AbilityRef::new(
                    baylee_core::ids::CardIndex::new(ondu_cleric()),
                    0
                ),
                answer: Some(StandingAnswer::Yes),
            },
            PlayerAction::SetStandingAnswer {
                ability: baylee_core::ids::AbilityRef::new(
                    baylee_core::ids::CardIndex::new(ondu_cleric()),
                    baylee_core::ids::AbilityRef::ENTERS
                ),
                answer: Some(StandingAnswer::No),
            },
        ]
    );
}

/// The reserved indices address abilities that are not listed on the
/// card, so a round trip through the store and over the engine link must
/// not confuse them with ability 0.
#[test]
fn reserved_ability_handles_survive_the_store() {
    let stored = vec![store::StandingAnswer {
        card: ondu_cleric(),
        ability: baylee_core::ids::AbilityRef::MIRACLE,
        yes: true,
    }];
    let json = serde_json::to_string(&stored).expect("serializes");
    let back: Vec<store::StandingAnswer> = serde_json::from_str(&json).expect("round trips");
    assert_eq!(back, stored);
    let actions = baylee_engine_server::standing_answers(&standing_payload(&back));
    let PlayerAction::SetStandingAnswer { ability, .. } = &actions[0] else {
        panic!("expected a standing answer")
    };
    assert!(!ability.is_listed_ability());
}

/// A seat with no account — the house playing an empty chair — has no
/// remembered answers, and must not send the engine something it cannot
/// parse in place of that.
#[test]
fn no_answers_is_an_empty_list_the_engine_can_read() {
    assert!(baylee_engine_server::standing_answers(&standing_payload(&[])).is_empty());
    assert!(baylee_engine_server::standing_answers(b"[]").is_empty());
}

/// A store written before standing answers existed still loads.
///
/// The file is no longer where the gateway keeps anything — it is what a
/// first start against an empty database takes over — so the claim is the
/// importer's now, and `baylee_db::import` is where the rest of it is
/// tested. This stays because the property is the gateway's: a gateway
/// started against a two-release-old file must not lose the accounts in it.
#[test]
fn an_older_store_file_still_loads() {
    let old = r#"{"accounts":{},"tokens":{},"decks":{}}"#;
    let legacy = baylee_db::import::read_legacy(old).expect("older store loads");
    assert!(legacy.automation.is_empty());
}

fn deck_named(cards: &[&str], commander: Option<&str>) -> store::Deck {
    store::Deck {
        id: "d".into(),
        account_id: "a".into(),
        name: "a commander deck".into(),
        cards: cards.iter().map(|s| (*s).to_string()).collect(),
        sideboard: Vec::new(),
        commander: commander.map(ToString::to_string),
        sleeve: None,
        playmat: None,
        updated_at: 0,
    }
}

/// A commander is one of the deck's rows on the way in — the builder
/// seats it among them on purpose — and exactly one card on the way out.
/// Copying it rather than moving it would shuffle a second Katara into
/// the library while the first one sat in the command zone.
#[test]
fn a_commander_is_moved_out_of_the_deck_list_and_not_copied() {
    let deck = deck_named(
        &["1 Katara, the Fearless", "3 Forest"],
        Some("Katara, the Fearless"),
    );
    let Ok(loaded) = loaded_deck(&deck) else {
        panic!("the rows and the name both resolve")
    };
    let katara = baylee_cards::decks::by_name("Katara, the Fearless").expect("in the pool");

    assert_eq!(loaded.commanders.len(), 1, "one commander");
    assert_eq!(loaded.commanders[0].index, katara);
    assert!(
        loaded.main.iter().all(|c| c.index != katara),
        "and no second copy left in the library"
    );
    assert_eq!(loaded.main.len(), 3, "the three Forests, and nothing else");
}

/// A deck posted straight to the API need not list its commander among
/// the rows, so a name that matches nothing there still seats one.
#[test]
fn a_commander_with_no_row_of_its_own_is_still_seated() {
    let deck = deck_named(&["3 Forest"], Some("Katara, the Fearless"));
    let Ok(loaded) = loaded_deck(&deck) else {
        panic!("the rows and the name both resolve")
    };
    assert_eq!(loaded.commanders.len(), 1, "one commander");
    assert_eq!(loaded.main.len(), 3, "and the deck list is untouched");
}
