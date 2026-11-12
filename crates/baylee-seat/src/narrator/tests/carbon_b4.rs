//! What the Carbon seat was offered wrong in block 4 (12.11.2026), on a real
//! engine: a payment window's one plan tapped the land with the scarce
//! colour where Forests paid.

use super::carbon::{picked, send};
use super::*;
use crate::wake::tests::played::position;
use baylee_core::ids::CardIndex;
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::Session;
use baylee_view::LogTail;

fn told(request: &Request) -> String {
    Narrator::new(&request.context)
        .wake(request, &[], None)
        .text
}

fn card(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).unwrap_or_else(|| panic!("{name} is not in the pool"))
}

/// Seat 0's untapped permanents named `name`, in the view's order.
fn untapped(session: &Session, name: &str) -> Vec<ObjectId> {
    let card = card(name);
    session
        .observe_view(ME)
        .battlefield
        .iter()
        .filter(|o| o.controller == ME && o.card.is_some_and(|c| c.index == card))
        .filter(|o| !o.status.contains(baylee_view::ObjectStatus::TAPPED))
        .map(|o| o.id)
        .collect()
}

/// The question seat 0 is asked now, with its note, as the bridge builds it.
fn asked_with_note(session: &Session) -> Request {
    let (pending, view) = session.view_for_with_context(ME).expect("seat 0 is asked");
    let mut asked = request(view, pending, LogTail::default());
    asked.note = session.question_note(ME);
    asked
}

/// The option lines of `text` that start with `label`, trimmed of their ids.
fn options<'a>(text: &'a str, label: &str) -> Vec<&'a str> {
    text.lines()
        .map(str::trim_start)
        .filter_map(|line| line.split_once("  ").map(|(_, rest)| rest))
        .filter(|rest| rest.starts_with(label))
        .collect()
}

/// Carbon b4, attempt 4, q240-q247: Nimbus Swimmer (`{X}{G}{U}`) cast at
/// X = 3 after a Forest and an Island were tapped, so `{G}{U}` floats and
/// the window owes `{3}{U}{G}` with four Forests and the other Island
/// untapped, Double Major (`{G}{U}`) in hand. The pool pays its own colours
/// and three Forests the `{3}` (CR 601.2g, 601.2h, 107.3a); the one plan
/// offered tapped two Forests and the Island, which left no blue for Double
/// Major. The first plan now taps three Forests, and the plan that taps the
/// Island is offered beside it, since it leaves different mana up.
#[test]
fn an_owed_payment_keeps_the_scarce_island_untapped_and_offers_the_other_way() {
    let (mut session, view, _) = position(
        &[
            "Forest", "Forest", "Forest", "Forest", "Forest", "Island", "Island",
        ],
        &["Nimbus Swimmer", "Double Major"],
        &[],
    );
    let forest = untapped(&session, "Forest")[0];
    let island = untapped(&session, "Island")[0];
    for land in [forest, island] {
        session
            .act_slice(ME, PlayerAction::ActivateManaAbility { source: land }, 0)
            .expect("a land's mana");
    }
    let swimmer = view
        .hand
        .iter()
        .find(|held| held.card.index == card("Nimbus Swimmer"))
        .expect("the Swimmer in hand")
        .id;
    session
        .act_slice(ME, PlayerAction::CastSpell { card: swimmer }, 0)
        .expect("{G}{U} floats for the Swimmer at X = 0");
    let x = asked_with_note(&session);
    assert!(
        matches!(x.pending, Pending::ChooseNumber { .. }),
        "X is asked: {:?}",
        x.pending
    );
    session
        .act_slice(ME, PlayerAction::ChooseNumber(3), 0)
        .expect("X = 3");

    let window = asked_with_note(&session);
    assert_eq!(
        window.view.owed,
        Some(baylee_core::mana::ManaPayment::Fixed(
            "{3}{G}{U}".parse().unwrap()
        )),
        "the window owes the whole cost"
    );
    let text = told(&window);
    let pays = options(&text, "Pay {3}{U}{G}");
    let island_tag = tag(untapped(&session, "Island")[0]);
    let first = pays
        .first()
        .unwrap_or_else(|| panic!("a plan pays it: {text}"));
    assert_eq!(
        first.matches("Forest").count(),
        3,
        "three Forests pay the {{3}}: {first}"
    );
    assert!(
        !first.contains(&island_tag),
        "the first plan keeps the Island: {first}"
    );
    assert!(
        pays.iter().skip(1).any(|other| other.contains(&island_tag)),
        "the plan that taps the Island is offered beside it: {pays:?}"
    );

    let act = picked(&window, first);
    send(&mut session, act);
    let after = session.observe_view(ME);
    assert!(
        after.stack.iter().any(|o| o.id == swimmer),
        "the Swimmer is cast for X = 3"
    );
    assert_eq!(
        untapped(&session, "Island").len(),
        1,
        "the Island is still up for Double Major"
    );
    assert_eq!(untapped(&session, "Forest").len(), 1);
}
