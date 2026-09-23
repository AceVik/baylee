//! The arrange question: cards put into places, in an order, at once.
//!
//! "Put the rest on the bottom in any order", "put them back in any order" —
//! and, later, scry and surveil and the piles of Fact or Fiction — are one
//! question with different destinations. A [`Pending::Arrange`] names the
//! cards and the piles they may go into; the answer lists every card exactly
//! once, pile by pile. Library piles are listed **top to bottom**, the way
//! the library will lie once the cards are in it.
//!
//! [`Pending::Arrange`]: super::Pending::Arrange

use baylee_core::ids::ObjectId;

/// Where one pile of a [`Pending::Arrange`](super::Pending::Arrange) puts
/// the cards placed in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ArrangePlace {
    /// On top of the library the cards were looked at in. Listed top to
    /// bottom: the first card is the new top card.
    LibraryTop,
    /// On the bottom of that library. Also listed top to bottom, so the last
    /// card is the bottom card — both library piles read the way the library
    /// will lie.
    LibraryBottom,
}

/// One destination of a [`Pending::Arrange`](super::Pending::Arrange): where,
/// how many, and whether the order inside it is the player's to choose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArrangePile {
    /// Where the cards go.
    pub place: ArrangePlace,
    /// The fewest cards this pile may hold.
    pub min: u32,
    /// The most cards this pile may hold.
    pub max: u32,
    /// Whether the order inside the pile is the player's to choose ("in any
    /// order"). An unordered pile's order is read as nothing at all.
    pub ordered: bool,
}

impl ArrangePile {
    /// A pile that must take every one of `n` cards, in an order the player
    /// chooses: "put them back in any order".
    #[must_use]
    pub const fn all_of(place: ArrangePlace, n: u32) -> Self {
        Self {
            place,
            min: n,
            max: n,
            ordered: true,
        }
    }
}

/// Why a [`Pending::Arrange`](super::Pending::Arrange) is asked (UI hint).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ArrangePrompt {
    /// Put cards in an order without choosing where they go: "the rest on
    /// the bottom in any order", "put them back in any order".
    Order,
}

/// The arrangement a player with no preference gives: the cards in the order
/// they were offered, each pile filled to its minimum in turn and what is
/// left into the first pile with room.
///
/// `None` when no arrangement is legal at all, which no question the engine
/// asks ever is — the bounds of its piles always add up around the cards.
#[must_use]
pub fn default_arrangement(
    cards: &[ObjectId],
    piles: &[ArrangePile],
) -> Option<Vec<Vec<ObjectId>>> {
    let mut out: Vec<Vec<ObjectId>> = vec![Vec::new(); piles.len()];
    let mut rest = cards.iter().copied();
    for (pile, spec) in out.iter_mut().zip(piles) {
        pile.extend(rest.by_ref().take(spec.min as usize));
    }
    for (pile, spec) in out.iter_mut().zip(piles) {
        let room = (spec.max as usize).saturating_sub(pile.len());
        pile.extend(rest.by_ref().take(room));
    }
    let filled = out
        .iter()
        .zip(piles)
        .all(|(pile, spec)| pile.len() >= spec.min as usize);
    (rest.next().is_none() && filled).then_some(out)
}

/// What is wrong with `answer` as an arrangement of `cards` into `piles`, or
/// `None` when it is a legal one.
///
/// One pile per destination, every card in exactly one of them, and each
/// pile within its bounds. The order inside an unordered pile is not read.
#[must_use]
pub fn arrangement_fault(
    cards: &[ObjectId],
    piles: &[ArrangePile],
    answer: &[Vec<ObjectId>],
) -> Option<&'static str> {
    if answer.len() != piles.len() {
        return Some("not one pile per destination");
    }
    if answer
        .iter()
        .zip(piles)
        .any(|(pile, spec)| !(spec.min as usize..=spec.max as usize).contains(&pile.len()))
    {
        return Some("a pile outside its bounds");
    }
    let mut placed: Vec<ObjectId> = answer.concat();
    let mut offered = cards.to_vec();
    placed.sort_unstable();
    offered.sort_unstable();
    (placed != offered).then_some("not every offered card exactly once")
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

    #[test]
    fn the_default_fills_minimums_first_and_then_the_first_pile_with_room() {
        let cards = [obj(1), obj(2), obj(3)];
        // Scry 3's shape: the top takes anything, the bottom anything.
        let scry = [
            pile(ArrangePlace::LibraryTop, 0, 3),
            pile(ArrangePlace::LibraryBottom, 0, 3),
        ];
        assert_eq!(
            default_arrangement(&cards, &scry),
            Some(vec![cards.to_vec(), vec![]]),
            "with no minimum anywhere, every card stays where it was offered"
        );
        // A bottom that must take two: its minimum is met before the top
        // gets what is left.
        let two_down = [
            pile(ArrangePlace::LibraryTop, 0, 1),
            pile(ArrangePlace::LibraryBottom, 2, 2),
        ];
        assert_eq!(
            default_arrangement(&cards, &two_down),
            Some(vec![vec![obj(3)], vec![obj(1), obj(2)]])
        );
        let every = [ArrangePile::all_of(ArrangePlace::LibraryBottom, 3)];
        assert_eq!(
            default_arrangement(&cards, &every),
            Some(vec![cards.to_vec()])
        );
    }

    #[test]
    fn the_default_is_none_when_the_bounds_cannot_hold_the_cards() {
        let cards = [obj(1), obj(2), obj(3)];
        assert_eq!(
            default_arrangement(&cards, &[pile(ArrangePlace::LibraryTop, 0, 2)]),
            None,
            "a pile of at most two cannot take three"
        );
        assert_eq!(
            default_arrangement(&cards, &[pile(ArrangePlace::LibraryTop, 4, 4)]),
            None,
            "a pile of at least four cannot be filled from three"
        );
    }

    #[test]
    fn an_answer_is_every_card_once_in_piles_within_their_bounds() {
        let cards = [obj(1), obj(2), obj(3)];
        let piles = [
            pile(ArrangePlace::LibraryTop, 0, 3),
            pile(ArrangePlace::LibraryBottom, 1, 3),
        ];
        assert_eq!(
            arrangement_fault(&cards, &piles, &[vec![obj(3)], vec![obj(2), obj(1)]]),
            None
        );
        assert!(
            arrangement_fault(&cards, &piles, &[vec![obj(1), obj(2), obj(3)]]).is_some(),
            "one pile short"
        );
        assert!(
            arrangement_fault(&cards, &piles, &[vec![obj(1), obj(2), obj(3)], vec![]]).is_some(),
            "the bottom must take one"
        );
        assert!(
            arrangement_fault(&cards, &piles, &[vec![obj(1)], vec![obj(1), obj(2)]]).is_some(),
            "a card twice and another not at all"
        );
        assert!(
            arrangement_fault(&cards, &piles, &[vec![obj(1)], vec![obj(2), obj(9)]]).is_some(),
            "a card that was not offered"
        );
    }
}
