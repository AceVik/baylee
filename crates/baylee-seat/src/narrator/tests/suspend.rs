//! Suspend from the hand (CR 116.2f, 702.62a): paid from the pool, or with
//! its mana still in the lands, tapped first and then suspended.

use super::*;
use baylee_core::mana::ManaCost;

fn vision_and_islands(islands: &[u32]) -> PlayerView {
    let lands: Vec<PublicObject> = islands.iter().map(|&slot| card(slot, "Island")).collect();
    let mut view = ViewBuilder::new(2).with_battlefield(0, lands).build();
    view.hand = vec![in_hand(61, "Ancestral Vision")];
    view
}

/// Carbon block 3, attempt 30, seat 1, q322: ten untapped Islands and
/// Ancestral Vision in hand, and the options were a1-a5 and pass. The card
/// prints no mana cost (CR 202.1b), so its suspend is the only way it ever
/// leaves the hand. The engine names it with its cost
/// (`LegalActions::unpaid_suspend`); the seat offers the taps, then the
/// suspend.
#[test]
fn a_suspend_the_lands_can_pay_is_offered_with_its_taps() {
    let view = vision_and_islands(&[70, 71]);
    let pending = Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            mana_abilities: vec![id(70), id(71)],
            unpaid_suspend: vec![(id(61), ManaCost::parse("{U}"))],
            ..Default::default()
        }),
    };
    let request = request(view, pending, LogTail::default());
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    let line = wake
        .text
        .lines()
        .find(|line| line.contains("Suspend Ancestral Vision #61 for {U}"))
        .unwrap_or_else(|| panic!("the suspend is offered:\n{}", wake.text));
    assert!(line.contains("(taps "), "{line}");
    let pick = line.split_whitespace().next().unwrap().to_string();
    let answer = wake
        .menu
        .resolve(&Decision {
            ask: Some("q12".into()),
            pick: vec![pick],
            ..Default::default()
        })
        .unwrap();
    assert!(
        matches!(&answer.act, Act::Taps { steps, then: PlayerAction::Suspend { card } }
            if *card == id(61) && steps.len() == 1),
        "{:?}",
        answer.act
    );
}

/// With the {U} floating the engine offers the suspend itself, and the
/// option says what it costs and what it does.
#[test]
fn a_suspend_paid_from_the_pool_names_its_cost() {
    let mut view = vision_and_islands(&[]);
    view.seats[0].mana_pool.blue = 1;
    let pending = Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            suspendable: vec![id(61)],
            ..Default::default()
        }),
    };
    let request = request(view, pending, LogTail::default());
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    let line = wake
        .text
        .lines()
        .find(|line| line.contains("Suspend Ancestral Vision #61 for {U}"))
        .unwrap_or_else(|| panic!("the suspend is offered:\n{}", wake.text));
    assert!(line.contains("paid from your pool"), "{line}");
    assert!(line.contains("4 time counters"), "{line}");
}

/// A seat whose only play is a suspend the lands pay is woken for it: the
/// window is not one with nothing but mana in it.
#[test]
fn a_reachable_suspend_is_a_play() {
    let view = vision_and_islands(&[70]);
    let legal = LegalActions {
        can_pass: true,
        mana_abilities: vec![id(70)],
        unpaid_suspend: vec![(id(61), ManaCost::parse("{U}"))],
        ..Default::default()
    };
    assert!(crate::wake::offering(&view, &legal));
}
