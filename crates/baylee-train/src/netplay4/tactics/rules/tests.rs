//! Each group on a real table: the engine sets the board, the seat's own view is
//! what the rule reads.
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::preset::{AIProfile, FormatId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;

use super::TacticRules4;

const ME: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

/// A table with these hands and boards, at the seat's first main phase.
fn table(
    my_hand: &[&str],
    my_board: &[&str],
    their_board: &[&str],
) -> (baylee_gamehost::Session, Pending, PlayerView) {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(915, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = FormatId::Freeform;
    preset.modifiers.clear();
    preset.house_rules.starting_player = Some(0);
    let card = |name: &str| {
        let mut entry = preset.seats[0].deck[0];
        entry.card = baylee_cards::decks::by_name(name).unwrap();
        entry.rule_print = None;
        entry.choices = baylee_core::card_metadata::DeckChoices::default();
        entry
    };
    let hand: Vec<_> = my_hand.iter().map(|name| card(name)).collect();
    let mine: Vec<_> = my_board.iter().map(|name| card(name)).collect();
    let theirs: Vec<_> = their_board.iter().map(|name| card(name)).collect();
    for (seat, spec) in preset.seats.iter_mut().enumerate() {
        spec.commanders.clear();
        spec.starting_hand = Some(if seat == 0 { hand.clone() } else { vec![] });
        spec.starting_battlefield = if seat == 0 {
            mine.clone()
        } else {
            theirs.clone()
        };
    }
    let mut session = baylee_gamehost::Session::new_recorded(&preset, "rule-group-test").unwrap();
    session.take_over(ME);
    session.take_over(OTHER);
    for _ in 0..64 {
        if let Pending::Priority { player, .. } = session.pending()
            && *player == ME
        {
            let (pending, view) = session.view_for_with_context(ME).unwrap();
            if view.active == ME && view.phase == baylee_view::Phase::FirstMain {
                return (session, pending, view);
            }
        }
        let player = session.pending().asked().unwrap();
        let action = match session.pending() {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers {
                attackers: Vec::new(),
            },
            other => panic!("unexpected setup decision {other:?}"),
        };
        session.act(player, action).unwrap();
    }
    panic!("first main not reached");
}

fn card_in_hand(view: &PlayerView, name: &str) -> ObjectId {
    view.hand.iter().find(|card| card.name == name).unwrap().id
}

fn groups(rules: &str, view: &PlayerView, pending: &Pending, action: &PlayerAction) -> Vec<String> {
    TacticRules4::parse(rules)
        .unwrap()
        .audit(view, pending, action)
        .into_iter()
        .map(|hit| hit.0.to_owned())
        .collect()
}

#[test]
fn a_gate_no_permanent_opens_is_void() {
    let (_session, pending, view) = table(&["Pyroblast"], &["Mountain"], &["Plains"]);
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Pyroblast"),
    };
    assert_eq!(groups("void", &view, &pending, &cast), ["void"]);
    let (_session, pending, view) = table(&["Pyroblast"], &["Mountain"], &["Air Elemental"]);
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Pyroblast"),
    };
    assert!(groups("void", &view, &pending, &cast).is_empty());
}

#[test]
fn damage_counted_on_a_board_without_it_is_void() {
    let (_session, pending, view) = table(&["Typhoon"], &["Forest"], &["Plains"]);
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Typhoon"),
    };
    assert_eq!(groups("void", &view, &pending, &cast), ["void"]);
    let (_session, pending, view) = table(&["Typhoon"], &["Forest"], &["Island"]);
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Typhoon"),
    };
    assert!(groups("void", &view, &pending, &cast).is_empty());
}

#[test]
fn a_sweep_of_the_seats_own_board_backfires() {
    let (_session, pending, view) = table(
        &["Pest Control"],
        &["Llanowar Elves", "Llanowar Elves"],
        &["Grizzly Bears"],
    );
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Pest Control"),
    };
    assert_eq!(groups("mass", &view, &pending, &cast), ["mass"]);
}

#[test]
fn a_lord_is_no_gift_on_a_board_of_the_seats_own() {
    let (_session, pending, view) = table(
        &["Goblin King"],
        &["Mountain", "Goblin Piker", "Goblin Piker"],
        &["Plains"],
    );
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Goblin King"),
    };
    assert!(groups("gift", &view, &pending, &cast).is_empty());
}

#[test]
fn a_lord_that_walks_their_goblins_through_my_mountain_is_a_gift() {
    let (_session, pending, view) = table(
        &["Goblin King"],
        &["Mountain", "Grizzly Bears", "Grizzly Bears"],
        &["Goblin Piker", "Goblin Piker", "Goblin Piker"],
    );
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Goblin King"),
    };
    assert_eq!(groups("gift", &view, &pending, &cast), ["gift"]);
}

#[test]
fn no_group_on_changes_nothing() {
    let rules = TacticRules4::parse("").unwrap();
    assert!(!rules.any());
    assert!(TacticRules4::parse("all").unwrap().any());
    assert!(TacticRules4::parse("nonsense").is_err());
}
