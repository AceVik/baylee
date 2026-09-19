//! Two questions about a card's sides, and why they are two.
//!
//! A client asks about a card's sides for two unrelated reasons, and for a
//! long time one `bool` answered both: `PoolCard::two_faced`, computed as
//! `def.faces.len() > 1`. That is a count of what *this build compiled*,
//! which is neither question, and it was wrong in both directions — twelve
//! cards were offered a back they do not have, and the count is not what
//! either consumer meant even where it agreed (#115).
//!
//! # A back to show
//!
//! [`has_back_image`] is whether Scryfall serves a second picture for the
//! printing this pool pinned. It is what the hover overlay and the deck
//! builder's preview are really asking, because the URL they would build is
//! Scryfall's `back` shelf and the answer to a card with no back is a 404.
//! Read from `image_uris`, which Scryfall puts at exactly one of two levels —
//! on the card for one piece of cardboard, on each face for two.
//!
//! # A double-faced card
//!
//! [`double_faced`] is CR 712.1: "a Magic card face on one side and either a
//! Magic card face or half of an oversized card face on the other. (It does
//! not have a Magic card back.)", in three kinds — nonmodal, modal, and meld.
//! That is what the deck builder's `double-faced` filter means, and it is a
//! claim about the printed card rather than about what this client can draw.
//! An Adventure and a Split are not double-faced under it: CR 715.1 makes an
//! adventurer card a two-part *frame*, and CR 709.1 says in as many words
//! that "the back of a split card is the normal Magic card back".
//!
//! # They are not the same set
//!
//! A **meld** card is double-faced and has no back image, because Scryfall
//! models a meld back as a card of its own rather than as a face of the
//! component. That is the whole of the difference in this pool, and it is why
//! one predicate could not have served both.

use crate::generated_sides::{BACK_IMAGE, DOUBLE_FACED};
use baylee_core::ids::CardIndex;

/// Whether Scryfall serves a second picture for this card's pinned printing.
///
/// The question to ask before offering to turn a card over, or before
/// building a URL on the `back` shelf.
#[must_use]
pub fn has_back_image(index: CardIndex) -> bool {
    BACK_IMAGE.binary_search(&index).is_ok()
}

/// Whether this is a double-faced card in the sense CR 712.1 gives the words.
///
/// A claim about the printing, not about this client's assets — see
/// [`has_back_image`] for the other one.
#[must_use]
pub fn double_faced(index: CardIndex) -> bool {
    DOUBLE_FACED.binary_search(&index).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{CardIndex, double_faced, has_back_image};
    use crate::generated_sides::{BACK_IMAGE, DOUBLE_FACED};

    /// Both tables are sorted, which is what the reader's binary search
    /// assumes and nothing else would notice being wrong: an unsorted table
    /// answers `false` for some of its own rows and never errors.
    #[test]
    fn both_tables_are_sorted_and_hold_no_card_twice() {
        for (name, rows) in [
            ("BACK_IMAGE", &BACK_IMAGE[..]),
            ("DOUBLE_FACED", &DOUBLE_FACED[..]),
        ] {
            assert!(
                rows.windows(2).all(|pair| pair[0] < pair[1]),
                "{name} is not strictly ascending, so its binary search is a coin toss"
            );
        }
    }

    /// Every row of both tables is a card this build actually compiled.
    ///
    /// The tables are written from the pool, so a row naming a card that is
    /// not in it means the generator and the registry have come apart — which
    /// is exactly what the two-phase gap looks like if the second run is
    /// forgotten.
    #[test]
    fn every_row_names_a_card_in_this_pool() {
        for (name, rows) in [
            ("BACK_IMAGE", &BACK_IMAGE[..]),
            ("DOUBLE_FACED", &DOUBLE_FACED[..]),
        ] {
            for index in rows {
                assert!(
                    crate::by_index(*index).is_some(),
                    "{name} names {index:?}, which this pool does not compile"
                );
            }
        }
    }

    /// The two questions are asked of different sets, and the difference is
    /// the finding.
    ///
    /// A meld card is double-faced under CR 712.1 and has no back image,
    /// because Scryfall keeps a meld back as a card of its own. If these two
    /// tables ever became equal, one of the predicates would be decoration
    /// and the next person would collapse them back into one bit — which is
    /// how this started. Bounded on both sides so neither a shrunken reader
    /// nor a runaway one passes.
    #[test]
    fn a_back_image_and_a_double_faced_card_are_different_questions() {
        assert!(
            BACK_IMAGE.len() > 50 && DOUBLE_FACED.len() > 50,
            "only {} with a back and {} double-faced — the generator read almost nothing",
            BACK_IMAGE.len(),
            DOUBLE_FACED.len()
        );
        let only_dfc: Vec<CardIndex> = DOUBLE_FACED
            .iter()
            .copied()
            .filter(|index| !has_back_image(*index))
            .collect();
        assert!(
            !only_dfc.is_empty(),
            "every double-faced card has a back image, so one of these two tables is not \
             answering its own question"
        );
        for index in only_dfc {
            assert!(
                double_faced(index),
                "{index:?} is in DOUBLE_FACED and the reader says it is not"
            );
        }
    }

    /// Everything with a back is a double-faced card, and not the reverse.
    ///
    /// The forward half is CR 712.1's own claim — a face on each side *is*
    /// what makes a card double-faced — and the generator refuses to write a
    /// row that breaks it, so this is the reader's copy of that refusal. The
    /// reverse fails on meld, which is the whole reason the two tables exist.
    #[test]
    fn a_back_image_implies_a_double_faced_card_and_not_the_other_way() {
        for index in &BACK_IMAGE {
            assert!(
                double_faced(*index),
                "{index:?} has a back and is not a double-faced card, which CR 712.1 does not \
                 allow — the generator should have refused to write this row"
            );
        }
        assert!(
            DOUBLE_FACED.len() > BACK_IMAGE.len(),
            "the two tables are the same size, so one of them is not being computed"
        );
    }

    /// A meld card is double-faced and has no back image.
    ///
    /// The card that separates the two questions: CR 712.1 names meld as one
    /// of its three kinds, and Scryfall keeps a meld back as a card of its
    /// own rather than as a face, so there is nothing at the `back` shelf to
    /// fetch. Named rather than derived, because this is the *example* — a
    /// derived one would still pass if the pool lost both meld cards.
    #[test]
    fn a_meld_card_is_double_faced_with_nothing_to_turn_to() {
        let index = crate::decks::by_name("Hanweir Battlements").expect("in the pool");
        assert!(double_faced(index), "CR 712.1 names meld cards");
        assert!(
            !has_back_image(index),
            "Scryfall serves no back for a meld component"
        );
    }

    /// A card with one printed face is in neither table.
    ///
    /// The counter-test to the one above: both predicates could be `true`
    /// everywhere and every assertion so far would still pass.
    #[test]
    fn an_ordinary_card_has_neither() {
        let index = crate::decks::by_name("Lightning Bolt").expect("in the pool");
        assert!(!has_back_image(index), "Lightning Bolt has no back");
        assert!(!double_faced(index), "Lightning Bolt has one face");
    }

    /// A transforming card has both, which is the ordinary case and the one
    /// the twelve false positives were being treated as.
    #[test]
    fn a_transforming_card_has_both() {
        let index = crate::decks::by_name("Agadeem's Awakening").expect("in the pool");
        assert!(
            has_back_image(index),
            "a modal double-faced card has a back"
        );
        assert!(double_faced(index), "CR 712.1 names modal DFCs");
    }

    /// An Adventure prints two names on one piece of card, so it has neither.
    ///
    /// This is the case `faces.len() > 1` got wrong: two compiled faces, no
    /// second side, and a client that built a back URL for it got a 404 —
    /// precisely what the old field's own doc said it existed to prevent.
    #[test]
    fn an_adventure_is_two_names_on_one_side() {
        let index = crate::decks::by_name("Murderous Rider").expect("in the pool");
        assert!(
            crate::by_index(index).is_some_and(|def| def.faces.len() > 1),
            "the premise: this card compiles two faces"
        );
        assert!(!has_back_image(index), "an Adventure has no second picture");
        assert!(
            !double_faced(index),
            "CR 715.1 makes an adventurer card a two-part frame, not a second side"
        );
    }
}
