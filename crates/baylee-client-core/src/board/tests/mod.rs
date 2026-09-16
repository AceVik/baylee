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
//! Every non-test item — `model`, `crowded_model`, `bolt_at_bears`, the two
//! widths — stays in this file. That is not tidiness but visibility: a child
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

const WIDE: f32 = 40.0;

/// A row barely wider than one card, which is where merging lives.
///
/// Identical permanents merge only once they would have to overlap, so a
/// test *about* merging has to be given a row that cannot hold its cards
/// — otherwise it draws them all separately and asserts nothing. Every
/// test below that is about what stays apart when things merge uses this
/// rather than [`WIDE`].
const CROWDED: f32 = 2.0;

fn model(view: &PlayerView) -> BoardModel {
    BoardModel::from_view(view, Openings::none(), |_| WIDE, Registry::none())
}

fn crowded_model(view: &PlayerView) -> BoardModel {
    BoardModel::from_view(view, Openings::none(), |_| CROWDED, Registry::none())
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
