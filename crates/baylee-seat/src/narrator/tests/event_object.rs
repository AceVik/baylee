//! A stack ability that is about an object without targeting it.

use super::*;
use baylee_core::ids::{AbilityRef, DamageSourceRef};
use baylee_view::{RulesFace, StackItem, TargetRef};

/// An ability of `card`'s on the stack, number `index` of its abilities,
/// from `source` and carrying `about` as its targets, as the host shows it.
fn ability(slot: u32, source: u32, card: &str, index: u32, about: u32) -> PublicObject {
    let card = identity(card).index;
    let mut object = token(slot, 0, "", 0, 0);
    object.stack_item = Some(StackItem::Ability {
        source: id(source),
        ability: Some(AbilityRef::new(card, index)),
        text: None,
        rules: (index != AbilityRef::SYNTHETIC).then_some(RulesFace { card, face: 0 }),
        token: None,
    });
    object.targets = vec![TargetRef::Object(DamageSourceRef {
        object: id(about),
        version: 0,
    })];
    object
}

fn stack_line(object: PublicObject) -> String {
    let (mut view, log) = board();
    let slot = object.id.slot();
    view.stack.push(object);
    let request = request(view.clone(), priority(&view), log);
    let wake = Narrator::new(&request.context).wake(&request, &[], None);
    wake.text
        .lines()
        .find(|line| line.trim_start().starts_with(&format!("#{slot} ")))
        .unwrap_or_else(|| panic!("#{slot} is on the stack:\n{}", wake.text))
        .to_string()
}

/// NEW-17: Spirit Shackle's "put a -0/-2 counter on it" is about the
/// enchanted creature that became tapped, which the card's script binds as
/// `TargetSpec::EventObject`; exalted's "that creature" the same; undying
/// returns the card that died (a rule's own trigger). None of them targets
/// (CR 115.1), and the stack line said "targets", so models reported them
/// as targeting (agy attempts 9 and 108, codex attempt 145).
#[test]
fn an_ability_about_its_event_object_does_not_say_it_targets() {
    // Spirit Shackle #98 on Prodigal Sorcerer #31: its second ability.
    let shackle = stack_line(ability(130, 98, "Spirit Shackle", 1, 31));
    // Ignoble Hierarch #43's exalted, for the Llanowar Elves #30.
    let exalted = stack_line(ability(131, 43, "Ignoble Hierarch", 1, 30));
    // Young Wolf #23's undying, for Young Wolf #23.
    let undying = stack_line(ability(132, 23, "Young Wolf", AbilityRef::SYNTHETIC, 23));
    for line in [&shackle, &exalted, &undying] {
        assert!(line.contains(" · about "), "{line}");
        assert!(line.contains("(not a target)"), "{line}");
        assert!(!line.contains("targets"), "{line}");
    }
    // Prodigal Sorcerer's "deals 1 damage to any target" targets.
    let pinger = stack_line(ability(133, 31, "Prodigal Sorcerer", 0, 45));
    assert!(pinger.contains(" · targets "), "{pinger}");
}
