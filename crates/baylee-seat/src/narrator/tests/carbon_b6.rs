//! What the Carbon seat was offered or told wrong in block 6 (12.11.2026),
//! on a real engine: a cast that tapped the only Mountain an `{R}` ability on
//! the board was waiting for.

use super::*;
use crate::wake::reachable;
use crate::wake::tests::played::position;
use baylee_core::ids::CardIndex;
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

/// Seat 0's permanents named `name`, in the view's order.
fn mine(session: &Session, name: &str) -> Vec<ObjectId> {
    let card = card(name);
    session
        .observe_view(ME)
        .battlefield
        .iter()
        .filter(|o| o.controller == ME && o.card.is_some_and(|c| c.index == card))
        .map(|o| o.id)
        .collect()
}

fn asked(session: &Session) -> Request {
    let (pending, view) = session.view_for(ME).expect("seat 0 is asked");
    request(view, pending, LogTail::default())
}

/// Carbon b6, segment 2, attempt 57, seat 1, q262: Sabertooth Nishoba
/// (`{4}{W}{G}`) in hand, seven lands (a white source, a Swamp, a Mountain,
/// four Forests) and Thornscape Apprentice on the battlefield, whose
/// "`{R}`, `{T}`: Target creature gains first strike" only the Mountain can
/// pay. The player chooses what pays (CR 601.2g, 601.2h): the white source,
/// the Swamp and the four Forests, the Mountain left up. The one plan
/// offered tapped the Mountain and left a Forest, because a land was
/// valued only by the colours of the cards in hand.
#[test]
fn a_cast_leaves_the_only_mountain_up_for_an_ability_on_the_board() {
    let (session, view, legal) = position(
        &[
            "Plains",
            "Swamp",
            "Mountain",
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Thornscape Apprentice",
        ],
        &["Sabertooth Nishoba"],
        &[],
    );
    let apprentice = mine(&session, "Thornscape Apprentice")[0];
    assert!(
        legal
            .unpaid_abilities
            .iter()
            .any(|&(source, _, _)| source == apprentice),
        "the Apprentice's abilities wait only for mana: {:?}",
        legal.unpaid_abilities
    );
    let mountain = mine(&session, "Mountain")[0];
    let reach = reachable(&view, &legal);
    let nishoba = reach
        .iter()
        .find(|r| r.card.index == card("Sabertooth Nishoba"))
        .expect("six lands pay the Nishoba");
    assert_eq!(nishoba.plan.steps.len(), 6, "{:?}", nishoba.plan);
    assert!(
        nishoba
            .plan
            .steps
            .iter()
            .all(|step| step.source != mountain),
        "the Mountain stays up for the Apprentice's {{R}}: {:?}",
        nishoba.plan
    );

    let text = told(&asked(&session));
    let cast = text
        .lines()
        .find(|line| line.contains("Cast Sabertooth Nishoba"))
        .unwrap_or_else(|| panic!("the Nishoba is offered: {text}"));
    assert!(!cast.contains(&tag(mountain)), "{cast}");
}
