//! What the board model decides, one question per file.
//!
//! # Where a test sits
//!
//! A test goes with **the decision it is about**: where a card lands in a
//! row is `lanes`, which seat a row belongs to is `seats`, what the stack
//! shows is `stack`, where a card came from is `provenance`, what a gap in
//! a row is for is `openings`, and what the model promises every caller is
//! `contracts`. The three files named after a *thing* on the mat — `piles`,
//! `fan`, `command_slots` — are that thing's own behaviour, and `fan` and
//! `command_slots` keep their module path because they were already nested
//! modules here.
//!
//! # What stays here, and why it has to
//!
//! Every non-test item — `model`, `bolt_at_bears` and the rest — stays in
//! this file. That is not tidiness but visibility: a child
//! module reaches its parent's private items through `use super::*`, and a
//! **sibling** reaches nothing at all. A helper that moved into one part
//! would be invisible to the other seven.

mod command_slots;
mod contracts;
mod fan;
mod lanes;
mod openings;
mod piles;
mod provenance;
mod seats;
mod stack;

use super::*;

use crate::test_support::{ViewBuilder, printed, token};

use baylee_cards_dsl::KeywordSet;
use baylee_core::ids::{AbilityRef, CardIndex, Defender, PrintRef};

use baylee_view::{AttackerView, BlockerView, CardIdentity, StackItem, StackText};

/// One chair on the roster, by the two facts that say who is answering for it.
///
/// Positional rather than a builder, because the pair is the whole point: a
/// caller that could set one and forget the other is exactly the shape
/// [`SeatRole`] exists to keep out of the rest of the client.
fn identity(player: u8, is_ai: bool, away: bool) -> baylee_view::SeatIdentity {
    baylee_view::SeatIdentity {
        player: PlayerId::new(player),
        display_name: format!("Seat {player}"),
        is_ai,
        away,
        team: None,
    }
}

fn model(view: &PlayerView) -> BoardModel {
    BoardModel::from_view(view, Openings::none(), &[], Registry::none())
}

/// A spell on the stack and the permanent it is pointed at.
fn bolt_at_bears() -> PlayerView {
    let bears = printed(1, 1, "Grizzly Bears", 11);
    let mut bolt = printed(2, 0, "Lightning Bolt", 22);
    bolt.types = TypeSet::INSTANT;
    bolt.power = None;
    bolt.toughness = None;
    bolt.stack_item = Some(StackItem::Spell);
    bolt.targets = vec![TargetRef::Object(ObjectId::new(1, 0))];
    ViewBuilder::new(2)
        .with_battlefield(1, [bears])
        .with_stack(vec![bolt])
        .build()
}
