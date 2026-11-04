//! Echo and a pact's price on the stack (a language model reported a Keldon
//! Champion's echo missing from the upkeep's stack, and it was).
//! They reach a seat as ordinary abilities, named by the sentence the card
//! prints, and the question they ask as they resolve is the tax's or the
//! pact's.

use super::*;
use baylee_cards_dsl::AbilityDef;
use baylee_core::ids::AbilityRef;
use baylee_core::mana::ManaCost;
use baylee_engine::choice::YesNoPrompt;
use baylee_view::{RulesFace, StackItem, StackText};

/// Ability `index` of `card` on the stack, seat 0's, as the host shows it.
fn on_stack(slot: u32, source: u32, card: &str, index: u32) -> PublicObject {
    let card = identity(card).index;
    let line = baylee_cards::lines::ability_line(card, 0, index).expect("it has a sentence");
    let mut object = token(slot, 0, "", 0, 0);
    object.stack_item = Some(StackItem::Ability {
        source: id(source),
        ability: Some(AbilityRef::new(card, index)),
        text: Some(StackText {
            face: 0,
            line: line.line,
            of: line.of,
        }),
        rules: Some(RulesFace { card, face: 0 }),
        token: None,
    });
    object
}

fn echo_of(name: &str) -> u32 {
    baylee_cards::by_index(identity(name).index)
        .and_then(|card| {
            card.abilities
                .iter()
                .position(|ability| matches!(ability, AbilityDef::Echo { .. }))
        })
        .and_then(|index| u32::try_from(index).ok())
        .expect("it prints echo")
}

fn wake_with(object: PublicObject, pending: impl FnOnce(&PlayerView) -> Pending) -> String {
    let (mut view, log) = board();
    view.phase = baylee_view::Phase::Beginning;
    view.step = baylee_view::Step::Upkeep;
    view.stack.push(object);
    let pending = pending(&view);
    let request = request(view, pending, log);
    Narrator::new(&request.context)
        .wake(&request, &[], None)
        .text
}

#[test]
fn echo_on_the_stack_says_what_it_is_and_what_it_costs() {
    let echo = echo_of("Keldon Champion");
    let text = wake_with(on_stack(130, 30, "Keldon Champion", echo), priority);
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with("#130 "))
        .unwrap_or_else(|| panic!("the echo is on the stack:\n{text}"));
    assert!(line.contains("Echo {2}{R}{R}"), "{line}");
    assert!(
        line.contains("if this came under your control"),
        "the reminder says what it does: {line}"
    );
    assert!(!line.contains("targets"), "{line}");
}

#[test]
fn a_pacts_price_on_the_stack_says_what_it_costs_and_what_it_risks() {
    let text = wake_with(on_stack(131, 61, "Pact of Negation", 1), priority);
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with("#131 "))
        .unwrap_or_else(|| panic!("the price is on the stack:\n{text}"));
    assert!(
        line.contains("pay {3}{U}{U}") && line.contains("lose the game"),
        "{line}"
    );
}

/// The echo's question is the one any "unless you pay" asks, beside the
/// ability that asks it.
#[test]
fn the_echo_asks_for_its_cost_as_it_resolves() {
    let echo = echo_of("Keldon Champion");
    let text = wake_with(on_stack(130, 30, "Keldon Champion", echo), |_| {
        Pending::YesNo {
            player: ME,
            prompt: YesNoPrompt::PayMana {
                cost: ManaCost::parse("{2}{R}{R}"),
            },
            source: Some(AbilityRef::new(identity("Keldon Champion").index, echo)),
        }
    });
    assert!(text.contains("Pay {2}{R}{R}?"), "{text}");
    assert!(text.contains("Echo {2}{R}{R}"), "{text}");
}

#[test]
fn a_pacts_price_warns_of_the_loss_as_it_resolves() {
    let text = wake_with(on_stack(131, 61, "Pact of Negation", 1), |_| {
        Pending::YesNo {
            player: ME,
            prompt: YesNoPrompt::PayPact {
                cost: ManaCost::parse("{3}{U}{U}"),
            },
            source: None,
        }
    });
    assert!(
        text.contains("Pay {3}{U}{U} for the pact? If you don't, you lose the game."),
        "{text}"
    );
}
