//! Where each scrolled row of the battlefield stands.
//!
//! A row scrolls rather than fanning past legibility, or rather than letting
//! a merged card's count badge lie on a neighbour's print (the owner, 25.09):
//! [`crate::layout::LanePacking::window`] shows a run of whole cards and the
//! rest of the row is not drawn, and a scrollbar under the row says that
//! there is more. This is the one number per row that says which run: the
//! first card shown. It moves a whole card at a time, by the wheel, and it
//! follows the card cursor, so a card the keyboard walks onto is always one
//! the player can see.

use crate::board::BoardModel;
use crate::layout::{LaneKind, TableLayout};
use baylee_core::ids::{ObjectId, PlayerId};
use std::collections::BTreeMap;

/// How much wheel travel moves a row one card, in logical pixels: a line of
/// a mouse wheel, as the hand counts one ([`crate::layout`] has no pixels;
/// the renderer converts a line to this many).
pub const ROW_STEP: f32 = 60.0;

/// One row of one seat.
pub type RowKey = (PlayerId, LaneKind);

/// The first card each scrolled row shows.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct RowScroll {
    first: BTreeMap<RowKey, usize>,
    /// The hovered card last brought into view, so a row the wheel has
    /// moved since is not pulled back to it on every frame.
    followed: Option<ObjectId>,
    /// Wheel travel not yet worth a card, and the row it was for.
    travel: Option<(RowKey, f32)>,
}

impl RowScroll {
    /// The first card `row` shows, before the packing clamps it.
    #[must_use]
    pub fn first(&self, row: RowKey) -> usize {
        self.first.get(&row).copied().unwrap_or(0)
    }

    /// Moves `row` by `pixels` of wheel travel, positive towards the row's
    /// later cards, a whole card per [`ROW_STEP`]. Returns whether the
    /// shown run moved.
    pub fn wheel(
        &mut self,
        board: &BoardModel,
        layout: &TableLayout,
        row: RowKey,
        pixels: f32,
    ) -> bool {
        let Some(packing) = packing_of(board, layout, row) else {
            return false;
        };
        let carried = match self.travel {
            Some((at, travel)) if at == row => travel,
            _ => 0.0,
        };
        let travel = carried + pixels;
        let cards = (travel / ROW_STEP).trunc();
        self.travel = Some((row, travel - cards * ROW_STEP));
        if cards == 0.0 || !packing.overflowing {
            return false;
        }
        let before = packing.window(self.first(row)).shown.start;
        #[allow(clippy::cast_possible_truncation)] // a handful of cards
        let moved = before
            .saturating_add_signed(cards as isize)
            .min(packing.last_first());
        self.first.insert(row, moved);
        moved != before
    }

    /// Brings `hovered` into view if a row hides it and it was not the card
    /// last brought into view, and forgets rows that no longer scroll.
    ///
    /// Called once a frame, before the table is placed. The pointer can only
    /// hover what is drawn, so a hidden hovered card is the card cursor's,
    /// and the cursor walks the whole row, hidden cards and all.
    pub fn follow(&mut self, board: &BoardModel, layout: &TableLayout, hovered: Option<ObjectId>) {
        self.first.retain(|&row, _| {
            packing_of(board, layout, row).is_some_and(|packing| packing.overflowing)
        });
        if hovered == self.followed {
            return;
        }
        self.followed = hovered;
        let Some(object) = hovered else {
            return;
        };
        let Some((row, index)) = row_of(board, object) else {
            return;
        };
        let Some(packing) = packing_of(board, layout, row) else {
            return;
        };
        if packing.overflowing {
            let first = packing.reveal(self.first(row), index);
            self.first.insert(row, first);
        }
    }
}

/// The row `object` is drawn in, and where in it: a card tucked under
/// another (#305) is where its host is.
#[must_use]
pub fn row_of(board: &BoardModel, object: ObjectId) -> Option<(RowKey, usize)> {
    board.pods.iter().find_map(|pod| {
        pod.lanes.iter().find_map(|lane| {
            lane.groups
                .iter()
                .position(|group| {
                    group
                        .with_attached()
                        .any(|card| card.members.contains(&object) || card.representative == object)
                })
                .map(|index| ((pod.player, lane.kind), index))
        })
    })
}

/// How `row` packs on this table, if the seat and the row are on it.
#[must_use]
pub fn packing_of(
    board: &BoardModel,
    layout: &TableLayout,
    (player, kind): RowKey,
) -> Option<crate::layout::LanePacking> {
    let slot = layout.slot(player)?;
    let pod = board.pods.iter().find(|pod| pod.player == player)?;
    let lane = pod.lanes.iter().find(|lane| lane.kind == kind)?;
    Some(lane.pack(slot))
}

#[cfg(test)]
mod tests;
