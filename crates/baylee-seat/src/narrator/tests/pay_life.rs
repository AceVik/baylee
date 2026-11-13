//! A life payment for one permanent, as the seat is told it, on a real
//! engine: Cleansing's "for each land, destroy that land unless any player
//! pays 1 life" asks land by land (CR 608.2d).

use super::carbon_b4::{asked_with_note, told, untapped};
use super::*;
use crate::wake::tests::played::position;
use baylee_engine::choice::{PlayerAction, YesNoPrompt};

/// Carbon b8, segment 3, attempt 27, seat 0, q163-q173: every question of
/// Cleansing read "Pay 1 life?", and the seat paid for ten lands without
/// knowing whose each was. The question names the land and whose it is,
/// and so do its answers.
#[test]
fn cleansing_asks_for_a_named_land() {
    let (mut session, view, _) = position(
        &["Plains", "Plains", "Plains", "Forest"],
        &["Cleansing"],
        &["Mountain"],
    );
    for plains in untapped(&session, "Plains") {
        session
            .act_slice(ME, PlayerAction::ActivateManaAbility { source: plains }, 0)
            .expect("a Plains' white");
    }
    session
        .act_slice(
            ME,
            PlayerAction::CastSpell {
                card: view.hand[0].id,
            },
            0,
        )
        .expect("{W}{W}{W} pays Cleansing");
    for _ in 0..12 {
        match session.pending().clone() {
            Pending::YesNo { player, .. } if player == ME => break,
            Pending::Priority { player, .. } => {
                session
                    .act_slice(player, PlayerAction::PassPriority, 0)
                    .expect("a pass");
            }
            other => panic!("on the way to Cleansing's question: {other:?}"),
        }
    }
    let asked = asked_with_note(&session);
    let Pending::YesNo {
        prompt:
            YesNoPrompt::PayLife {
                amount: 1,
                permanent: Some(land),
            },
        ..
    } = asked.pending.clone()
    else {
        panic!("a life payment for a land: {:?}", asked.pending);
    };
    let object = asked.view.object(land).expect("the land is in view");
    let whose = if object.controller == ME {
        "your"
    } else {
        "P2's"
    };
    let named = format!("{} {}", object.name, tag(land));
    let text = told(&asked);
    assert!(
        text.contains(&format!("Pay 1 life for {whose} {named}?")),
        "{text}"
    );
    assert!(text.contains(&format!("pay 1 life for {named}")), "{text}");
    assert!(text.contains(&format!("don't pay for {named}")), "{text}");
}
