mod answering;
mod filtering;
mod offers;
mod opening;
mod sheet;
mod sorting;
mod tabs;

use super::*;

// The registry here is the empty one, and deliberately: a zone browser
// lists cards in hidden zones, and a card that arrives in one is a new
// object with no memory of its previous existence (CR 400.7), so whatever
// it was copying on the battlefield it is not copying in a graveyard.
// Nothing a browser draws is ever wearing another card's face.
use crate::board::{BoardModel, Openings, Registry};

use crate::test_support::{ViewBuilder, printed};

use baylee_engine::choice::{ChoicePrompt, Pending, TargetPrompt};

fn me() -> PlayerId {
    PlayerId::new(0)
}

/// What is ticked, in tab order, as a list a test can read.
///
/// A `Vec` and not the set itself: the empty case is what "Alle" means
/// and `vec![]` says that in the assertion, where `BTreeSet::new()` says
/// only that the set is empty.
fn ticks(b: &Browser) -> Vec<BrowseZone> {
    b.ticked().iter().copied().collect()
}

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

/// Everything a client can already click without the browser: the
/// battlefield as drawn cards, and the seat's own hand.
fn drawn_on_the_table(view: &PlayerView) -> Vec<ObjectId> {
    let board = BoardModel::from_view(view, Openings::none(), |_| 100.0, &[], Registry::none());
    let mut ids: Vec<ObjectId> = board
        .pods
        .iter()
        .flat_map(|p| p.lanes.iter())
        .flat_map(|l| l.groups.iter())
        .flat_map(|g| g.members.iter().copied())
        .collect();
    ids.extend(board.hand.iter().map(|c| c.id));
    ids
}
