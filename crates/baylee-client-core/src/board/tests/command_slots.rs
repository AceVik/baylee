//! The command zone is one zone drawn as one place per commander, and a
//! seat that has none is drawn no place at all.
//!
//! Three claims, and the middle one is the only one a reader might not
//! expect: nothing *reflows*. A pile stands where its own kind stands, so
//! a seat with a single commander shows one slot and bare table where the
//! second would be, rather than sliding the graveyard over to close the
//! gap. Commanders are fixed before the first turn (CR 903.3), so a
//! seat's set of slots is the same on the last turn as on the first.

use super::*;

fn slots(view: &PlayerView) -> Vec<PileKind> {
    zone_piles(view, PlayerId::new(0))
        .into_iter()
        .map(|p| p.kind)
        .collect()
}

#[test]
fn a_seat_with_no_commander_is_drawn_no_command_zone() {
    let view = ViewBuilder::new(2).build();
    assert_eq!(
        slots(&view),
        vec![PileKind::Library, PileKind::Graveyard, PileKind::Exile],
        "a deck with no commander has no zone to draw"
    );
}

#[test]
fn one_commander_is_one_slot_and_two_are_two() {
    let first = printed(1, 0, "Sidar Kondo", 11);
    let second = printed(2, 0, "Tana", 12);
    let one = ViewBuilder::new(2)
        .with_commanders(0, &[&first])
        .with_command(0, vec![first.clone()])
        .build();
    assert_eq!(
        slots(&one),
        vec![
            PileKind::Library,
            PileKind::Graveyard,
            PileKind::Exile,
            PileKind::Command
        ]
    );

    let two = ViewBuilder::new(2)
        .with_commanders(0, &[&first, &second])
        .with_command(0, vec![first.clone(), second.clone()])
        .build();
    assert_eq!(
        slots(&two),
        vec![
            PileKind::Library,
            PileKind::Graveyard,
            PileKind::Exile,
            PileKind::Command,
            PileKind::Command2
        ]
    );
}

/// Each partner lies on its own slot, and the second one is matched
/// by handle rather than by position: an `ObjectId` survives the
/// moves that make a card a new object (CR 400.7), so a partner that
/// dies, goes home and comes back down lands on the slot it left.
#[test]
fn each_partner_lies_on_its_own_slot() {
    let first = printed(1, 0, "Sidar Kondo", 11);
    let second = printed(2, 0, "Tana", 12);
    // Listed second-first, which is what a command zone looks like
    // after the first one has been cast and has come back.
    let view = ViewBuilder::new(2)
        .with_commanders(0, &[&first, &second])
        .with_command(0, vec![second.clone(), first.clone()])
        .build();
    let piles = zone_piles(&view, PlayerId::new(0));
    let at = |kind| {
        piles
            .iter()
            .find(|p| p.kind == kind)
            .expect("the slot is drawn")
            .clone()
    };
    assert_eq!(at(PileKind::Command).top, Some(first.id));
    assert_eq!(at(PileKind::Command).count, 1);
    assert_eq!(at(PileKind::Command2).top, Some(second.id));
    assert_eq!(at(PileKind::Command2).count, 1);
}

/// A commander that is on the battlefield leaves its slot empty, and
/// the slot is still there: the zone exists whether or not a card is
/// in it, the commander can return to it (CR 903.9), and the
/// uncovered mark is exactly the signal "your commander is out".
#[test]
fn a_commander_on_the_battlefield_leaves_its_slot_standing_and_empty() {
    let first = printed(1, 0, "Sidar Kondo", 11);
    let view = ViewBuilder::new(2).with_commanders(0, &[&first]).build();
    let piles = zone_piles(&view, PlayerId::new(0));
    let command = piles
        .iter()
        .find(|p| p.kind == PileKind::Command)
        .expect("the slot is still drawn");
    assert_eq!(command.count, 0);
    assert_eq!(command.top, None);
}

/// An emblem belongs to the first slot. It is in the command zone and
/// it is not a commander, so it goes where a seat with one commander
/// already looks.
#[test]
fn what_is_not_a_commander_lies_on_the_first_slot() {
    let first = printed(1, 0, "Sidar Kondo", 11);
    let second = printed(2, 0, "Tana", 12);
    let emblem = token(3, 0, "Emblem", 0, 0);
    let view = ViewBuilder::new(2)
        .with_commanders(0, &[&first, &second])
        .with_command(0, vec![first.clone(), emblem.clone(), second.clone()])
        .build();
    let piles = zone_piles(&view, PlayerId::new(0));
    let count = |kind| {
        piles
            .iter()
            .find(|p| p.kind == kind)
            .expect("the slot is drawn")
            .count
    };
    assert_eq!(count(PileKind::Command), 2, "the commander and the emblem");
    assert_eq!(count(PileKind::Command2), 1);
}

#[test]
fn commanders_remain_single_cards_when_hovered() {
    let first = printed(1, 0, "Sidar Kondo", 11);
    let second = printed(2, 0, "Tana", 12);
    let view = ViewBuilder::new(2)
        .with_commanders(0, &[&first, &second])
        .with_command(0, vec![first.clone(), second.clone()])
        .build();
    for pile in zone_piles(&view, PlayerId::new(0)) {
        if matches!(pile.kind, PileKind::Command | PileKind::Command2) {
            assert!(pile.fan.is_empty());
            assert_eq!(pile.fan_len(), 0);
            assert!(pile.top.is_some());
        }
    }
}
