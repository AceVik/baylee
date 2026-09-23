//! The answer to a `Pending::Arrange` while a player is building it.
//!
//! An arrangement is every offered card in exactly one row: one row per pile
//! the question names, and an **unplaced** row for cards not yet put
//! anywhere. A pile is listed the way the engine reads it — a library pile
//! top to bottom, whichever end of the library it goes to — so the first card
//! in the `LibraryTop` row is the new top card and the last card in the
//! `LibraryBottom` row is the new bottom card.
//!
//! The gesture is **tap, then place**, never a drag: tapping a card holds it,
//! tapping another card puts the held one in front of it, and tapping a row's
//! trailing slot puts it at the end of that row. A keyboard does the same
//! with a focus that walks every card, and nudges that move the held card one
//! step at a time. The model knows no renderer; the tray draws what it says.

use crate::i18n::Phrase;
use crate::interaction::SelectionOutcome;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::{ArrangePile, ArrangePlace};

/// One row of an arrangement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Row {
    /// The cards not yet placed. Drawn only while it holds something, or
    /// while the question began with cards in it.
    Unplaced,
    /// Pile `n` of the question, in the order the question lists its piles.
    Pile(usize),
}

/// A move of the held card by one step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nudge {
    /// One place earlier in its row: towards the top of a library pile.
    Earlier,
    /// One place later in its row.
    Later,
    /// To the end of the row above.
    PrevRow,
    /// To the end of the row below.
    NextRow,
}

/// Why an arrangement cannot be sent yet — the first reason, in row order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocker {
    /// This many cards are in no pile yet.
    Unplaced(usize),
    /// Pile `pile` holds `by` fewer cards than its minimum.
    Short {
        /// Which pile.
        pile: usize,
        /// How many more it needs.
        by: usize,
    },
}

/// An arrangement being built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arrangement {
    specs: Vec<ArrangePile>,
    /// The offered cards, in the order the question named them.
    dealt: Vec<ObjectId>,
    piles: Vec<Vec<ObjectId>>,
    unplaced: Vec<ObjectId>,
    /// Whether the unplaced row is drawn even while empty: a question that
    /// began with cards in it keeps its row, so emptying it does not move
    /// every other row up under the player's pointer.
    unplaced_row: bool,
    held: Option<ObjectId>,
    focus: Option<ObjectId>,
}

impl Arrangement {
    /// The arrangement a question starts from.
    ///
    /// Every card in the first pile that can take all of them, in the order
    /// they were offered — for a scry that is "leave them all on top, as
    /// they were", which is a complete answer the player may send at once.
    /// With no such pile the cards start unplaced.
    #[must_use]
    pub fn new(cards: &[ObjectId], specs: &[ArrangePile]) -> Self {
        let mut piles = vec![Vec::new(); specs.len()];
        let mut unplaced = Vec::new();
        match specs
            .iter()
            .position(|spec| spec.max as usize >= cards.len() && spec.min as usize <= cards.len())
        {
            Some(first) => piles[first] = cards.to_vec(),
            None => unplaced = cards.to_vec(),
        }
        let unplaced_row = !unplaced.is_empty();
        Self {
            specs: specs.to_vec(),
            dealt: cards.to_vec(),
            piles,
            unplaced,
            unplaced_row,
            held: None,
            focus: cards.first().copied(),
        }
    }

    /// Whether this arrangement answers the question `cards` into `specs` —
    /// the test for keeping a half-built arrangement when the same question
    /// is sent again.
    #[must_use]
    pub fn same_question(&self, cards: &[ObjectId], specs: &[ArrangePile]) -> bool {
        self.dealt == cards && self.specs == specs
    }

    /// The piles the question names.
    #[must_use]
    pub fn specs(&self) -> &[ArrangePile] {
        &self.specs
    }

    /// Every offered card, in the order the question named them.
    #[must_use]
    pub fn dealt(&self) -> &[ObjectId] {
        &self.dealt
    }

    /// The rows to draw, top to bottom.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        let piles = (0..self.specs.len()).map(Row::Pile);
        if self.unplaced_row || !self.unplaced.is_empty() {
            std::iter::once(Row::Unplaced).chain(piles).collect()
        } else {
            piles.collect()
        }
    }

    /// The cards in a row, first listed first.
    #[must_use]
    pub fn cards(&self, row: Row) -> &[ObjectId] {
        match row {
            Row::Unplaced => &self.unplaced,
            Row::Pile(n) => self.piles.get(n).map_or(&[], Vec::as_slice),
        }
    }

    /// Where a card is.
    #[must_use]
    pub fn slot(&self, id: ObjectId) -> Option<(Row, usize)> {
        self.rows().into_iter().find_map(|row| {
            self.cards(row)
                .iter()
                .position(|c| *c == id)
                .map(|at| (row, at))
        })
    }

    /// The card being moved, if any.
    #[must_use]
    pub const fn held(&self) -> Option<ObjectId> {
        self.held
    }

    /// The card the keyboard stands on.
    #[must_use]
    pub const fn focused(&self) -> Option<ObjectId> {
        self.focus
    }

    /// Whether `row` can take one more card from elsewhere.
    #[must_use]
    pub fn has_room(&self, row: Row) -> bool {
        match row {
            Row::Unplaced => true,
            Row::Pile(n) => self
                .specs
                .get(n)
                .is_some_and(|spec| self.piles[n].len() < spec.max as usize),
        }
    }

    /// Whether a tap on the end of `row` would put the held card there: a
    /// card is held, and the row has room for it or already holds it.
    #[must_use]
    pub fn can_place(&self, row: Row) -> bool {
        self.held.is_some_and(|held| {
            self.has_room(row) || self.slot(held).is_some_and(|(at, _)| at == row)
        })
    }

    /// What a row is called on screen: where its cards are going.
    #[must_use]
    pub fn label(&self, row: Row) -> Phrase {
        match row {
            Row::Unplaced => Phrase::ArrangeUnplaced,
            Row::Pile(n) => match self.specs.get(n).map(|spec| spec.place) {
                Some(ArrangePlace::LibraryBottom) => Phrase::ArrangeLibraryBottom,
                Some(ArrangePlace::Graveyard) => Phrase::ArrangeGraveyard,
                Some(ArrangePlace::LibraryTop) | None => Phrase::ArrangeLibraryTop,
            },
        }
    }

    /// A tap on a card: hold it, let it go, or put the held card in front
    /// of it.
    ///
    /// `Added` when a card was taken up or put down, `Removed` when the held
    /// card was let go where it was, `Full` when the held card cannot go
    /// into the tapped card's pile, and `Rejected` for a card that is not
    /// part of the question.
    pub fn toggle(&mut self, id: ObjectId) -> SelectionOutcome {
        if !self.dealt.contains(&id) {
            return SelectionOutcome::Rejected;
        }
        self.focus = Some(id);
        match self.held {
            None => {
                self.held = Some(id);
                SelectionOutcome::Added
            }
            Some(held) if held == id => {
                self.held = None;
                SelectionOutcome::Removed
            }
            Some(held) => {
                let Some((row, at)) = self.slot(id) else {
                    return SelectionOutcome::Rejected;
                };
                if !self.move_to(held, row, at) {
                    return SelectionOutcome::Full;
                }
                self.held = None;
                self.focus = Some(held);
                SelectionOutcome::Added
            }
        }
    }

    /// Puts the held card at the end of `row`. `false` when nothing is held
    /// or the row is full.
    pub fn place(&mut self, row: Row) -> bool {
        let Some(held) = self.held else {
            return false;
        };
        let end = self.cards(row).iter().filter(|c| **c != held).count();
        if !self.move_to(held, row, end) {
            return false;
        }
        self.held = None;
        self.focus = Some(held);
        true
    }

    /// Moves the held card one step. `false` when nothing is held or there
    /// is nowhere to go.
    pub fn nudge(&mut self, nudge: Nudge) -> bool {
        let Some(held) = self.held else {
            return false;
        };
        let Some((row, at)) = self.slot(held) else {
            return false;
        };
        match nudge {
            Nudge::Earlier if at > 0 => self.move_to(held, row, at - 1),
            Nudge::Later if at + 1 < self.cards(row).len() => self.move_to(held, row, at + 1),
            Nudge::PrevRow | Nudge::NextRow => {
                let rows = self.rows();
                let Some(here) = rows.iter().position(|r| *r == row) else {
                    return false;
                };
                // The next row with room, skipping a full one rather than
                // stopping at it: a scry's bottom pile never fills, but a
                // pile with a maximum of one is a wall a keyboard would
                // otherwise have no way past.
                let order: Vec<Row> = if nudge == Nudge::NextRow {
                    rows[here + 1..].to_vec()
                } else {
                    rows[..here].iter().rev().copied().collect()
                };
                let Some(to) = order.into_iter().find(|r| self.has_room(*r)) else {
                    return false;
                };
                let end = self.cards(to).len();
                self.move_to(held, to, end)
            }
            _ => false,
        }
    }

    /// Lets go of the held card, or — with nothing held — puts every card
    /// back where the question started. `true` when something changed.
    pub fn cancel(&mut self) -> bool {
        if self.held.take().is_some() {
            return true;
        }
        let fresh = Self::new(&self.dealt, &self.specs);
        let changed = fresh.piles != self.piles || fresh.unplaced != self.unplaced;
        *self = fresh;
        changed
    }

    /// Why the arrangement cannot be sent yet, or `None` when it can.
    #[must_use]
    pub fn blocker(&self) -> Option<Blocker> {
        if !self.unplaced.is_empty() {
            return Some(Blocker::Unplaced(self.unplaced.len()));
        }
        self.specs.iter().enumerate().find_map(|(pile, spec)| {
            let short = (spec.min as usize).saturating_sub(self.piles[pile].len());
            (short > 0).then_some(Blocker::Short { pile, by: short })
        })
    }

    /// The answer, when the arrangement is complete: one list per pile.
    #[must_use]
    pub fn answer(&self) -> Option<Vec<Vec<ObjectId>>> {
        self.blocker().is_none().then(|| self.piles.clone())
    }

    /// Walks the focus through every card, row by row, wrapping.
    pub fn cycle_focus(&mut self, delta: i32) -> Option<ObjectId> {
        let walk: Vec<ObjectId> = self
            .rows()
            .into_iter()
            .flat_map(|row| self.cards(row).to_vec())
            .collect();
        if walk.is_empty() {
            return None;
        }
        let here = self
            .focus
            .and_then(|f| walk.iter().position(|c| *c == f))
            .unwrap_or(0);
        let len = i64::try_from(walk.len()).unwrap_or(1);
        let next = (i64::try_from(here).unwrap_or(0) + i64::from(delta)).rem_euclid(len);
        self.focus = walk.get(usize::try_from(next).unwrap_or(0)).copied();
        self.focus
    }

    /// Where the focus sits among every card, as `(position, count)`.
    #[must_use]
    pub fn focus_position(&self) -> Option<(usize, usize)> {
        let walk: Vec<ObjectId> = self
            .rows()
            .into_iter()
            .flat_map(|row| self.cards(row).to_vec())
            .collect();
        let at = self.focus.and_then(|f| walk.iter().position(|c| *c == f))?;
        Some((at, walk.len()))
    }

    /// Takes `card` out of wherever it is and puts it at `at` in `row`,
    /// counted as if the card were not there. Refused when `row` is a
    /// different pile that is already full.
    fn move_to(&mut self, card: ObjectId, row: Row, at: usize) -> bool {
        let from = self.slot(card).map(|(r, _)| r);
        if from != Some(row) && !self.has_room(row) {
            return false;
        }
        if let Some(from) = from {
            self.row_mut(from).retain(|c| *c != card);
        }
        let target = self.row_mut(row);
        let at = at.min(target.len());
        target.insert(at, card);
        true
    }

    fn row_mut(&mut self, row: Row) -> &mut Vec<ObjectId> {
        match row {
            Row::Unplaced => &mut self.unplaced,
            Row::Pile(n) => &mut self.piles[n],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(n: u32) -> ObjectId {
        ObjectId::new(n, 0)
    }

    fn pile(place: ArrangePlace, min: u32, max: u32) -> ArrangePile {
        ArrangePile {
            place,
            min,
            max,
            ordered: true,
        }
    }

    /// Scry 3: the top and the bottom each take any number.
    fn scry() -> Arrangement {
        Arrangement::new(
            &[obj(1), obj(2), obj(3)],
            &[
                pile(ArrangePlace::LibraryTop, 0, 3),
                pile(ArrangePlace::LibraryBottom, 0, 3),
            ],
        )
    }

    #[test]
    fn a_scry_starts_with_every_card_on_top_as_it_was_and_may_be_sent_at_once() {
        let a = scry();
        assert_eq!(a.cards(Row::Pile(0)), &[obj(1), obj(2), obj(3)]);
        assert!(a.cards(Row::Pile(1)).is_empty());
        assert_eq!(
            a.rows(),
            vec![Row::Pile(0), Row::Pile(1)],
            "no unplaced row"
        );
        assert_eq!(
            a.answer(),
            Some(vec![vec![obj(1), obj(2), obj(3)], vec![]]),
            "keeping everything on top is an answer"
        );
    }

    #[test]
    fn a_held_card_goes_in_front_of_the_card_tapped_next() {
        let mut a = scry();
        assert_eq!(a.toggle(obj(3)), SelectionOutcome::Added);
        assert_eq!(a.held(), Some(obj(3)));
        assert_eq!(a.toggle(obj(1)), SelectionOutcome::Added);
        assert_eq!(a.held(), None, "putting it down lets go of it");
        assert_eq!(a.cards(Row::Pile(0)), &[obj(3), obj(1), obj(2)]);
    }

    #[test]
    fn tapping_the_held_card_again_lets_go_of_it_where_it_is() {
        let mut a = scry();
        a.toggle(obj(2));
        assert_eq!(a.toggle(obj(2)), SelectionOutcome::Removed);
        assert_eq!(a.held(), None);
        assert_eq!(a.cards(Row::Pile(0)), &[obj(1), obj(2), obj(3)]);
    }

    #[test]
    fn placing_puts_the_held_card_at_the_end_of_the_row() {
        let mut a = scry();
        a.toggle(obj(1));
        assert!(a.place(Row::Pile(1)), "the bottom has room");
        a.toggle(obj(3));
        assert!(a.place(Row::Pile(1)));
        assert_eq!(a.cards(Row::Pile(0)), &[obj(2)]);
        assert_eq!(
            a.cards(Row::Pile(1)),
            &[obj(1), obj(3)],
            "listed top to bottom: the card placed last is the bottom card"
        );
        assert!(!a.place(Row::Pile(0)), "nothing is held");
    }

    #[test]
    fn a_full_pile_refuses_a_card_from_elsewhere_and_reorders_its_own() {
        let mut a = Arrangement::new(
            &[obj(1), obj(2), obj(3)],
            &[
                pile(ArrangePlace::LibraryTop, 0, 3),
                pile(ArrangePlace::LibraryBottom, 0, 1),
            ],
        );
        a.toggle(obj(1));
        assert!(a.place(Row::Pile(1)));
        a.toggle(obj(2));
        assert!(!a.place(Row::Pile(1)), "a bottom of at most one is full");
        assert_eq!(
            a.toggle(obj(1)),
            SelectionOutcome::Full,
            "nor may it go in front of the card already there"
        );
        assert_eq!(a.held(), Some(obj(2)), "and it is still held");
        a.cancel();
        a.toggle(obj(3));
        assert_eq!(
            a.toggle(obj(2)),
            SelectionOutcome::Added,
            "within a pile is fine"
        );
        assert_eq!(a.cards(Row::Pile(0)), &[obj(3), obj(2)]);
    }

    #[test]
    fn nudges_walk_the_held_card_along_its_row_and_across_rows() {
        let mut a = scry();
        a.toggle(obj(1));
        assert!(!a.nudge(Nudge::Earlier), "already first");
        assert!(a.nudge(Nudge::Later));
        assert_eq!(a.cards(Row::Pile(0)), &[obj(2), obj(1), obj(3)]);
        assert!(a.nudge(Nudge::NextRow));
        assert_eq!(a.cards(Row::Pile(1)), &[obj(1)]);
        assert!(!a.nudge(Nudge::NextRow), "no row below the bottom");
        assert!(a.nudge(Nudge::PrevRow));
        assert_eq!(a.cards(Row::Pile(0)), &[obj(2), obj(3), obj(1)]);
        assert_eq!(a.held(), Some(obj(1)), "a nudge keeps holding");
    }

    #[test]
    fn a_question_no_pile_can_take_whole_starts_unplaced_and_is_blocked_until_placed() {
        // Fact or Fiction's separation, reduced: two piles of at most two.
        let mut a = Arrangement::new(
            &[obj(1), obj(2), obj(3)],
            &[
                pile(ArrangePlace::LibraryTop, 1, 2),
                pile(ArrangePlace::LibraryBottom, 1, 2),
            ],
        );
        assert_eq!(a.rows()[0], Row::Unplaced);
        assert_eq!(a.blocker(), Some(Blocker::Unplaced(3)));
        assert_eq!(a.answer(), None);
        for (card, row) in [(obj(1), 1), (obj(2), 1), (obj(3), 2)] {
            a.toggle(card);
            assert!(a.place(a.rows()[row]));
        }
        assert_eq!(a.blocker(), None);
        assert_eq!(a.rows()[0], Row::Unplaced, "the emptied row stays put");
        assert_eq!(a.answer(), Some(vec![vec![obj(1), obj(2)], vec![obj(3)]]));
    }

    #[test]
    fn a_pile_short_of_its_minimum_is_the_blocker() {
        let mut a = Arrangement::new(
            &[obj(1), obj(2)],
            &[
                pile(ArrangePlace::LibraryTop, 0, 2),
                pile(ArrangePlace::LibraryBottom, 1, 2),
            ],
        );
        assert_eq!(a.blocker(), Some(Blocker::Short { pile: 1, by: 1 }));
        a.toggle(obj(2));
        a.place(Row::Pile(1));
        assert_eq!(a.blocker(), None);
    }

    #[test]
    fn cancel_lets_go_first_and_resets_second() {
        let mut a = scry();
        a.toggle(obj(1));
        a.place(Row::Pile(1));
        a.toggle(obj(2));
        assert!(a.cancel(), "lets go of the held card");
        assert_eq!(a.cards(Row::Pile(1)), &[obj(1)], "and moves nothing");
        assert!(a.cancel(), "then puts everything back");
        assert_eq!(a, scry());
        assert!(!a.cancel(), "and a fresh arrangement has nothing to undo");
    }

    #[test]
    fn the_focus_walks_every_card_row_by_row_and_wraps() {
        let mut a = scry();
        a.toggle(obj(3));
        a.place(Row::Pile(1));
        assert_eq!(
            a.focused(),
            Some(obj(3)),
            "the focus follows what was placed"
        );
        assert_eq!(a.focus_position(), Some((2, 3)));
        assert_eq!(a.cycle_focus(1), Some(obj(1)), "wraps to the first row");
        assert_eq!(a.cycle_focus(-1), Some(obj(3)));
        assert_eq!(a.cycle_focus(-1), Some(obj(2)));
    }

    #[test]
    fn a_card_that_was_not_offered_is_refused() {
        let mut a = scry();
        assert_eq!(a.toggle(obj(9)), SelectionOutcome::Rejected);
        assert_eq!(a.held(), None);
    }

    #[test]
    fn the_same_question_is_recognised_and_a_different_one_is_not() {
        let a = scry();
        let specs = a.specs().to_vec();
        assert!(a.same_question(&[obj(1), obj(2), obj(3)], &specs));
        assert!(!a.same_question(&[obj(1), obj(2)], &specs));
        assert!(!a.same_question(&[obj(1), obj(2), obj(3)], &specs[..1]));
    }

    /// A pile's end takes the held card when there is room or the card is
    /// already in it — and nothing at all while no card is held.
    #[test]
    fn a_piles_end_takes_the_held_card_only_where_it_fits() {
        let mut a = Arrangement::new(
            &[obj(1), obj(2)],
            &[
                pile(ArrangePlace::LibraryTop, 0, 2),
                pile(ArrangePlace::Graveyard, 0, 1),
            ],
        );
        assert!(!a.can_place(Row::Pile(1)), "nothing is held");
        a.toggle(obj(1));
        assert!(a.can_place(Row::Pile(0)), "its own pile, to its end");
        assert!(a.can_place(Row::Pile(1)), "an empty pile of one");
        assert!(a.place(Row::Pile(1)));
        a.toggle(obj(2));
        assert!(!a.can_place(Row::Pile(1)), "the pile of one is full");
        assert!(!a.place(Row::Pile(1)), "and placing there is refused");
        a.toggle(obj(2));
        a.toggle(obj(1));
        assert!(a.can_place(Row::Pile(1)), "the card already in it may stay");
    }

    /// A row is named after where its cards go, and a pile nobody placed
    /// anything in is named all the same.
    #[test]
    fn every_row_is_named_after_where_its_cards_go() {
        let a = Arrangement::new(
            &[obj(1), obj(2)],
            &[
                pile(ArrangePlace::LibraryTop, 1, 1),
                pile(ArrangePlace::LibraryBottom, 0, 1),
                pile(ArrangePlace::Graveyard, 0, 1),
            ],
        );
        assert_eq!(a.rows()[0], Row::Unplaced, "no pile could take both");
        assert_eq!(a.label(Row::Unplaced), Phrase::ArrangeUnplaced);
        assert_eq!(a.label(Row::Pile(0)), Phrase::ArrangeLibraryTop);
        assert_eq!(a.label(Row::Pile(1)), Phrase::ArrangeLibraryBottom);
        assert_eq!(a.label(Row::Pile(2)), Phrase::ArrangeGraveyard);
    }
}
