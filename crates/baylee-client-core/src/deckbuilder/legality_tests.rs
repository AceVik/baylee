//! The deck's standing as the builder holds it, with real cards.

use super::legality::ChosenList;
use super::*;
use baylee_cards::formats::FREEFORM;
use baylee_cardtext::{
    CardLine, ListRules, ListStatus, PoolFacts, PoolFactsBody, PoolSet, Problem, Standing,
};
use std::sync::Arc;

/// A pool row for a real card, so the check reads the registry it reads in
/// a game.
fn real(name: &str) -> PoolCard {
    let index = baylee_cards::decks::by_name(name)
        .unwrap_or_else(|| panic!("{name} is in the pool"))
        .get();
    let def = baylee_cards::by_index(baylee_core::ids::CardIndex::new(index)).expect("a card");
    let basic = baylee_cards::formats::is_basic_land(def.index);
    let eligible = baylee_cards::decks::leader_of(def.index).is_some_and(|l| l.eligible);
    PoolCard {
        index,
        name: name.to_string(),
        english_name: name.to_string(),
        kinds: if basic {
            vec!["Land".to_string()]
        } else {
            vec!["Artifact".to_string()]
        },
        coverage: Coverage::Implemented,
        commander: eligible,
        basic_land: basic,
        oracle_id: def.oracle_id.to_string(),
        ..PoolCard::default()
    }
}

const POOL: [&str; 4] = ["Island", "Sol Ring", "Counterspell", "General Tazri"];

fn builder() -> DeckBuilder {
    let mut b = DeckBuilder::new();
    b.set_pool(POOL.iter().map(|name| real(name)).collect(), true);
    b.set_name("Test");
    b
}

fn index(name: &str) -> u32 {
    real(name).index
}

fn slot(b: &DeckBuilder, name: &str) -> usize {
    b.slot_of(name).expect("the card is in the pool")
}

fn add(b: &mut DeckBuilder, name: &str, copies: usize) {
    let at = slot(b, name);
    for _ in 0..copies {
        assert!(b.add(at, Zone::Main), "{name}");
    }
}

/// A table holding these verdicts in one format.
fn table(format: &str, verdicts: &[(&str, &str)]) -> Arc<PoolFacts> {
    let lines = verdicts
        .iter()
        .map(|(name, word)| CardLine {
            index: index(name),
            legalities: [(format.to_owned(), (*word).to_owned())].into(),
            printed: Vec::new(),
        })
        .collect();
    let set = PoolSet {
        code: "tst".to_owned(),
        name: "Test".to_owned(),
    };
    Arc::new(PoolFactsBody::assemble(vec![set], lines).into())
}

fn sixty_islands(b: &mut DeckBuilder) {
    add(b, "Island", 60);
}

#[test]
fn a_new_deck_keeps_to_no_format_until_it_has_a_leader() {
    let mut b = builder();
    assert_eq!(b.format(), FREEFORM);
    assert_eq!(b.standing(), Standing::Legal, "nothing to keep to");
    let overview = b.overview(Lang::En);
    assert_eq!(
        overview.headline,
        "No format chosen, so nothing is checked."
    );

    let tazri = slot(&b, "General Tazri");
    assert!(b.set_commander(tazri));
    assert_eq!(
        b.format(),
        "commander",
        "read off the leader, as the gateway does"
    );
}

#[test]
fn the_check_answers_every_edit() {
    let mut b = builder();
    b.set_pool_facts(table(
        "modern",
        &[("Island", "legal"), ("Sol Ring", "banned")],
    ));
    b.set_format("modern");
    assert_eq!(b.report().problems, [Problem::TooFew { have: 0, need: 60 }]);
    sixty_islands(&mut b);
    assert_eq!(b.standing(), Standing::Legal);
    let ring = slot(&b, "Sol Ring");
    assert!(b.add(ring, Zone::Main));
    assert_eq!(b.standing(), Standing::Illegal);
    assert_eq!(
        b.report().problems,
        [Problem::Banned {
            card: index("Sol Ring")
        }]
    );
    assert!(b.remove(ring, Zone::Main));
    assert_eq!(b.standing(), Standing::Legal, "and the edit back");
}

/// Each problem points at its card, and a row wears the word for it.
#[test]
fn a_problem_points_at_its_card_and_its_row_wears_a_badge() {
    let mut b = builder();
    b.set_pool_facts(table(
        "modern",
        &[("Island", "legal"), ("Sol Ring", "banned")],
    ));
    b.set_format("modern");
    sixty_islands(&mut b);
    let ring = slot(&b, "Sol Ring");
    b.add(ring, Zone::Main);
    let overview = b.overview(Lang::En);
    assert_eq!(overview.standing, Standing::Illegal);
    assert_eq!(overview.problems.len(), 1);
    assert_eq!(overview.problems[0].slot, Some(ring));
    assert!(overview.problems[0].text.contains("Sol Ring"));
    assert!(overview.problems[0].text.contains("Modern"));
    assert!(
        overview.headline.contains("one problem"),
        "{}",
        overview.headline
    );
    assert_eq!(b.badge(ring), Some(Phrase::BadgeBanned));
    assert_eq!(b.badge(slot(&b, "Island")), None);
    assert_eq!(
        b.card_problems(ring, Lang::En),
        vec![overview.problems[0].text.clone()]
    );
}

#[test]
fn the_overview_says_how_big_the_deck_is_against_the_format() {
    let mut b = builder();
    b.set_format("modern");
    add(&mut b, "Island", 12);
    assert_eq!(b.overview(Lang::En).count, "12 cards, at least 60");
    b.set_format("commander");
    assert_eq!(b.overview(Lang::En).count, "12 of 100 cards");
    b.set_format("oathbreaker");
    assert_eq!(b.overview(Lang::En).count, "12 cards");
}

/// Offline there is no table: the shape is still asked, the deck is never
/// called legal, and the overview says what could not be checked.
#[test]
fn without_the_table_a_deck_is_not_called_legal() {
    let mut b = builder();
    b.set_format("modern");
    sixty_islands(&mut b);
    assert_eq!(b.report().problems, []);
    assert_eq!(b.standing(), Standing::Unknown);
    let overview = b.overview(Lang::En);
    assert!(
        !overview.headline.starts_with("Legal"),
        "{}",
        overview.headline
    );
    assert!(
        overview.notices.iter().any(|n| n.contains("card table")),
        "{:?}",
        overview.notices
    );

    b.set_pool_facts(table("modern", &[("Island", "legal")]));
    assert_eq!(b.standing(), Standing::Legal, "the table settles it");
}

#[test]
fn a_table_that_says_nothing_about_a_card_keeps_the_deck_from_legal() {
    let mut b = builder();
    b.set_pool_facts(table("modern", &[("Island", "legal")]));
    b.set_format("modern");
    add(&mut b, "Island", 59);
    add(&mut b, "Sol Ring", 1);
    assert_eq!(b.standing(), Standing::Unknown);
    assert!(
        b.overview(Lang::En)
            .notices
            .iter()
            .any(|n| n.contains("One card could not be checked"))
    );
}

/// A leader sits among the main rows, and is counted once.
#[test]
fn a_commander_is_one_card_of_the_hundred() {
    let mut b = builder();
    b.set_pool_facts(table(
        "commander",
        &[("Island", "legal"), ("General Tazri", "legal")],
    ));
    b.set_format("commander");
    let tazri = slot(&b, "General Tazri");
    assert!(b.set_commander(tazri));
    add(&mut b, "Island", 99);
    assert_eq!(b.counts().main, 100);
    assert_eq!(b.report().problems, []);
    assert_eq!(b.standing(), Standing::Legal);
}

#[test]
fn a_list_is_held_up_to_the_deck_and_named() {
    let mut b = builder();
    b.set_format(FREEFORM);
    add(&mut b, "Sol Ring", 2);
    let def = baylee_cards::decks::by_name("Sol Ring")
        .and_then(baylee_cards::by_index)
        .expect("a card");
    let rules: ListRules = [(def.oracle_id.to_string(), ListStatus::Restricted)]
        .into_iter()
        .collect();
    b.set_list(Some(ChosenList {
        id: "l1".to_string(),
        name: "Kitchen table".to_string(),
        builtin: false,
        format: FREEFORM.to_string(),
        rules: Arc::new(rules),
    }));
    let overview = b.overview(Lang::En);
    assert_eq!(overview.standing, Standing::Illegal);
    assert!(overview.headline.contains("Kitchen table"));
    assert!(overview.problems[0].text.contains("Kitchen table"));
    assert_eq!(b.badge(slot(&b, "Sol Ring")), Some(Phrase::BadgeRestricted));
    b.set_list(None);
    assert_eq!(b.standing(), Standing::Legal);
}

/// A list keeps to its format, as a table holding it would ask, and an
/// official list goes by the format's name in the client's language.
#[test]
fn a_list_brings_its_format_to_the_check() {
    let mut b = builder();
    b.set_format("modern");
    add(&mut b, "Island", 60);
    b.hold_to(&baylee_cardtext::ListBody {
        row: baylee_cardtext::ListRow {
            id: "l1".to_string(),
            name: "commander".to_string(),
            format: "commander".to_string(),
            builtin: true,
            ..baylee_cardtext::ListRow::default()
        },
        entries: Vec::new(),
    });
    assert_eq!(b.format(), "modern", "the deck's own stands");
    assert!(
        b.report().problems.contains(&Problem::WrongSize {
            have: 60,
            need: 100
        }),
        "{:?}",
        b.report().problems
    );
    assert!(b.overview(Lang::En).headline.contains("Commander"));
}

/// `list:` answers once a gateway has told the lists, says so until then, and
/// goes at sign-out with what that gateway said.
#[test]
fn a_list_key_filters_the_pool_once_the_lists_are_told() {
    let mut b = builder();
    b.set_text("list:\"kitchen table\"");
    assert!(b.results().is_empty());
    assert_eq!(b.search_lacking(), Some(Phrase::FilterNoLists));

    let row = baylee_cardtext::ListRow {
        name: "Kitchen table".to_string(),
        format: FREEFORM.to_string(),
        indices: vec![index("Sol Ring")],
        ..baylee_cardtext::ListRow::default()
    };
    b.set_lists(vec![row], Lang::En);
    assert_eq!(b.search_lacking(), None);
    assert_eq!(b.results(), [slot(&b, "Sol Ring")], "the results moved");

    b.forget_pool();
    assert!(b.list_shelf().is_none());
}

/// The gear belongs to the search in front: the list editor's while one is
/// open, and the pool's panel and box stand as they were.
#[test]
fn the_gear_works_the_list_editors_search_while_it_is_open() {
    use crate::filterdialog::{Act, OFFERED};
    let mut b = builder();
    b.set_text("sol");
    let pool_results = b.results().to_vec();
    b.open_new_list("Mine");
    b.toggle_panel();
    assert!(b.panel().is_some(), "the editor's panel");
    assert!(b.pool_panel().is_none(), "and not the pool's");

    let at = OFFERED
        .iter()
        .position(|key| *key == crate::cardquery::Key::Name)
        .expect("the menu offers a name");
    b.filter_act(Act::Add(at));
    b.filter_act(Act::Edit(0));
    b.type_into_panel("sol");
    let open = b.list_editor().expect("open");
    assert_eq!(open.search.field.text(), "name:sol", "the editor's box");
    assert_eq!(open.results(), [slot(&b, "Sol Ring")], "and it answered");
    assert_eq!(b.text(), "sol", "the pool's box is left alone");
    assert_eq!(b.results(), pool_results);

    b.close_panel();
    assert!(b.panel().is_none());
    b.close_list();
    b.toggle_panel();
    assert!(b.pool_panel().is_some(), "with no editor it is the pool's");
}

/// An official list is read: its search and gear are not drawn, and the box
/// behind them writes nothing that could change it.
#[test]
fn a_read_only_list_takes_nothing_from_the_gear() {
    let mut b = builder();
    b.open_list(&baylee_cardtext::ListBody {
        row: baylee_cardtext::ListRow {
            id: "o1".to_string(),
            name: "modern".to_string(),
            format: "modern".to_string(),
            builtin: true,
            ..baylee_cardtext::ListRow::default()
        },
        entries: Vec::new(),
    });
    b.edit_list(|list, pool| {
        assert!(!list.put(&pool[0], Some(ListStatus::Banned)));
    });
    assert!(
        b.list_editor()
            .is_some_and(|list| list.entries().is_empty())
    );
}

/// A language change reads the lists again, and asks nothing of a client no
/// gateway has told any.
#[test]
fn a_new_language_keeps_what_the_lists_are() {
    let mut b = builder();
    b.relabel_lists(Lang::De);
    assert!(b.list_shelf().is_none(), "no shelf out of nothing");
    b.set_lists(Vec::new(), Lang::En);
    b.relabel_lists(Lang::De);
    assert!(b.list_shelf().is_some());
}

/// Illegal is told and never refused: the button stays live.
#[test]
fn an_illegal_deck_still_saves_with_its_format() {
    let mut b = builder();
    b.set_pool_facts(table("modern", &[("Sol Ring", "banned")]));
    b.set_format("modern");
    add(&mut b, "Sol Ring", 1);
    assert_eq!(b.standing(), Standing::Illegal);
    assert!(b.saveable());
    let Some(LobbyRequest::SaveDeck { format, .. }) = b.save() else {
        panic!("the deck saves");
    };
    assert_eq!(format.as_deref(), Some("modern"));
}

/// Choosing a format is an edit: it is saved with the deck. Taking what the
/// store says is not.
#[test]
fn a_format_chosen_is_an_edit_and_a_format_read_is_not() {
    let mut b = builder();
    b.load("d1", "Old", &["4 Island".to_string()], &[], &[]);
    b.keep_format("modern");
    assert!(!b.dirty());
    assert_eq!(b.format(), "modern");
    b.set_format("modern");
    assert!(!b.dirty(), "the same format is no change");
    b.set_format("legacy");
    assert!(b.dirty());
}

/// A deck that became illegal later is told so when it is opened.
#[test]
fn a_deck_opened_after_a_ban_is_told_so() {
    let mut b = builder();
    b.set_pool_facts(table(
        "modern",
        &[("Island", "legal"), ("Sol Ring", "banned")],
    ));
    let rows = ["59 Island".to_string(), "1 Sol Ring".to_string()];
    b.load("d1", "Old", &rows, &[], &[]);
    b.keep_format("modern");
    assert_eq!(b.standing(), Standing::Illegal);
    assert!(!b.dirty());
}

/// Rows waiting for the pool are not a deck anyone can call legal.
#[test]
fn a_deck_waiting_for_the_pool_is_not_legal_yet() {
    let mut b = DeckBuilder::new();
    b.load("d1", "Old", &["4 Island".to_string()], &[], &[]);
    b.keep_format(FREEFORM);
    assert_eq!(b.standing(), Standing::Unknown);
    assert!(
        b.overview(Lang::En)
            .notices
            .iter()
            .any(|n| n.contains("card pool"))
    );
    b.set_pool(POOL.iter().map(|name| real(name)).collect(), true);
    assert_eq!(b.standing(), Standing::Legal);
}

#[test]
fn a_picker_offers_none_first_and_then_what_the_table_holds() {
    let mut b = builder();
    let offline = b.format_choices();
    assert_eq!(offline[0], FREEFORM);
    assert!(offline.contains(&"modern".to_string()));
    assert!(
        !offline.contains(&"brawl".to_string()),
        "offline only what this build can shape"
    );
    let at = |choices: &[String], key: &str| choices.iter().position(|c| c == key);
    assert!(at(&offline, "standard") < at(&offline, "alchemy"));

    b.set_pool_facts(table("brawl", &[("Island", "legal")]));
    assert_eq!(b.format_choices(), [FREEFORM, "brawl"]);
}

#[test]
fn a_draft_keeps_the_format() {
    let mut b = builder();
    b.set_format("modern");
    add(&mut b, "Island", 1);
    let draft = b.draft();
    let mut other = builder();
    other.restore(&draft);
    assert_eq!(other.format(), "modern");
}

/// Forgetting the pool at sign-out forgets the table and the list with it:
/// the next gateway's verdicts are its own.
#[test]
fn signing_out_forgets_what_the_gateway_said() {
    let mut b = builder();
    b.set_pool_facts(table("modern", &[("Island", "legal")]));
    b.set_format("modern");
    b.forget_pool();
    assert!(b.pool_facts().is_none());
    assert_eq!(b.format(), FREEFORM);
    assert!(b.list().is_none());
}
