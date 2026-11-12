//! What the Carbon seat was offered or told wrong in block 6 (12.11.2026),
//! on a real engine where it can be: a cast that tapped the only Mountain an
//! `{R}` ability on the board was waiting for, and Ward Sliver's chosen
//! colour shown nowhere.

use super::carbon::{picked, send};
use super::*;
use crate::wake::reachable;
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

/// Ward Sliver chose blue as it entered: every Sliver has protection from
/// blue (CR 614.12, 702.16), and nothing the seat read said so, because the
/// view's `chosen_color` was never printed (Carbon b6, segment 2, attempt
/// 63, seat 0, q252). The permanent's line says it now, as it says a chosen
/// type or name.
#[test]
fn a_permanent_s_line_says_the_colour_chosen_as_it_entered() {
    let (mut session, view, _) = position(
        &["Plains", "Plains", "Plains", "Plains", "Plains"],
        &["Ward Sliver"],
        &[],
    );
    let sliver = view.hand[0].id;
    let request = asked(&session);
    let act = picked(&request, "Cast Ward Sliver");
    send(&mut session, act);
    for _ in 0..8 {
        match session.pending().clone() {
            Pending::ChooseColor { player, .. } => {
                session
                    .act_slice(
                        player,
                        PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Blue),
                        0,
                    )
                    .expect("blue");
            }
            Pending::Priority { player, .. }
                if session
                    .observe_view(ME)
                    .battlefield
                    .iter()
                    .any(|o| o.id == sliver) =>
            {
                assert_eq!(player, ME, "the Sliver resolved on seat 0's turn");
                break;
            }
            Pending::Priority { player, .. } => {
                session
                    .act_slice(player, PlayerAction::PassPriority, 0)
                    .expect("a pass");
            }
            other => panic!("the Sliver's cast asked {other:?}"),
        }
    }
    let request = asked(&session);
    assert_eq!(
        request.view.object(sliver).and_then(|o| o.chosen_color),
        Some(baylee_core::mana::ManaColor::Blue),
        "the view carries the colour"
    );
    let text = told(&request);
    let line = text
        .lines()
        .find(|line| line.contains(&format!("{} Ward Sliver", tag(sliver))))
        .unwrap_or_else(|| panic!("Ward Sliver on the battlefield: {text}"));
    assert!(line.contains("chosen color blue"), "{line}");
}

/// The other choice made as a permanent enters that the view carries and
/// the seat did not print: an opponent (Black Vise). Shown as the chosen
/// type and name are, beside the colour.
#[test]
fn a_permanent_s_line_says_the_opponent_and_colour_chosen_for_it() {
    let (mut view, log) = board();
    let serra = view
        .battlefield
        .iter_mut()
        .find(|o| o.id == id(45))
        .expect("Serra Angel");
    serra.chosen_opponent = Some(ME);
    serra.chosen_color = Some(baylee_core::mana::ManaColor::Green);
    let text = told(&request(view, priority(&board().0), log));
    let line = text
        .lines()
        .find(|line| line.contains("#45 Serra Angel"))
        .unwrap_or_else(|| panic!("Serra Angel: {text}"));
    assert!(line.contains("chosen opponent you"), "{line}");
    assert!(line.contains("chosen color green"), "{line}");
}
