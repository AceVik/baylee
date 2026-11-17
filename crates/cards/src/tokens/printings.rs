//! The printed token card a **generated** token wears the picture of.
//!
//! A token a reader wrote out of the card-script reference has no printing of
//! its own: [`TokenDef::scryfall_id`](baylee_cards_dsl::TokenDef) stays empty
//! in `generated_tokens.rs`, which only codegen writes. This table is the
//! other half, chosen by a tool that held each token against the printed
//! token cards and kept only exact matches — name, card types, supertypes,
//! subtypes, colours, power and toughness, and a rules box of nothing but
//! the token's own keywords. A token with abilities of its own is never in
//! it: nothing in the registry says those abilities in words to compare.
//!
//! Keyed on the ledger id, which is append only, and carrying the name, so
//! a row that ever pointed at the wrong token fails the tests below rather
//! than dressing one token in another's picture.

use super::by_token_id;

/// One generated token and the printed token card it wears.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TokenPrinting {
    /// The token's ledger id (its index in [`super::ALL`]).
    pub token: u16,
    /// The token's name, as the ledger holds it: checked, never read.
    pub name: &'static str,
    /// Scryfall's id of the printed token card.
    pub scryfall_id: &'static str,
    /// The printing's set code, for a person checking the row.
    pub set: &'static str,
    /// The printing's collector number, for the same person.
    pub number: &'static str,
}

/// Every chosen printing, in ledger order.
pub static PRINTINGS: &[TokenPrinting] = include!("printing_rows.rs");

/// The printed token card a generated token wears, if one was chosen.
#[must_use]
pub fn of(token: u16) -> Option<&'static str> {
    PRINTINGS
        .binary_search_by_key(&token, |row| row.token)
        .ok()
        .map(|at| PRINTINGS[at].scryfall_id)
}

/// The picture a token wears: its own printing, else the one chosen here.
///
/// The one door every reader of a token's picture goes through. Empty means
/// none, and the client draws the token's text face.
#[must_use]
pub fn picture(token: u16) -> Option<&'static str> {
    let def = by_token_id(token)?;
    if def.scryfall_id.is_empty() {
        of(token)
    } else {
        Some(def.scryfall_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated_tokens::GENERATED;

    /// A binary search over rows that are out of order finds nothing for
    /// some of them, silently; so the order is a test.
    #[test]
    fn the_rows_are_in_ledger_order_and_name_each_token_once() {
        for pair in PRINTINGS.windows(2) {
            assert!(
                pair[0].token < pair[1].token,
                "{} and {} are out of order or twice",
                pair[0].name,
                pair[1].name
            );
        }
    }

    /// Every row still names the token it was chosen for, and only a token a
    /// reader wrote: a hand-written token carries its own printing, and the
    /// ones with no picture on purpose (Tower items, the Snake, Jace) must
    /// never gain one from here.
    #[test]
    fn every_row_names_a_generated_token_by_its_own_name() {
        for row in PRINTINGS {
            let token = by_token_id(row.token)
                .unwrap_or_else(|| panic!("{}: no token has id {}", row.name, row.token));
            assert_eq!(
                token.name, row.name,
                "row {} names another token",
                row.token
            );
            assert!(
                GENERATED.iter().any(|g| std::ptr::eq(*g, token)),
                "{} (id {}) is hand-written and carries its own printing",
                row.name,
                row.token
            );
            assert!(
                token.abilities.is_empty(),
                "{} has abilities nobody compared with the printed card",
                row.name
            );
        }
    }

    /// The shape `baylee_client_core::images` accepts: a canonical,
    /// hyphenated, non-nil UUID. Anything else is a request that fails every
    /// time the token is drawn.
    #[test]
    fn every_row_is_a_canonical_uuid() {
        for row in PRINTINGS {
            let parsed = uuid::Uuid::parse_str(row.scryfall_id)
                .unwrap_or_else(|e| panic!("{}: {} is no UUID: {e}", row.name, row.scryfall_id));
            assert_eq!(
                parsed.hyphenated().to_string(),
                row.scryfall_id,
                "{}",
                row.name
            );
            assert!(!parsed.is_nil(), "{}", row.name);
            assert!(
                !row.set.is_empty() && !row.number.is_empty(),
                "{}",
                row.name
            );
        }
    }

    /// No picture is worn by two different tokens, here or across the
    /// hand-written half: the tool matches every characteristic, and the
    /// ledger holds no two equal tokens, so a shared id is a wrong row.
    #[test]
    fn no_two_tokens_wear_the_same_printed_card() {
        let mut ids: Vec<&str> = (0..super::super::ALL.len())
            .filter_map(|i| u16::try_from(i).ok())
            .filter_map(picture)
            .collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "two tokens share a printed card");
    }

    /// The door answers a token's own printing first, a chosen one second,
    /// and nothing for an id no token has.
    #[test]
    fn the_door_prefers_the_tokens_own_printing() {
        let zombie = super::super::token_id(&super::super::ZOMBIE_2_2_BLACK);
        assert_eq!(
            picture(zombie),
            Some(super::super::ZOMBIE_2_2_BLACK.scryfall_id)
        );
        for row in PRINTINGS {
            assert_eq!(picture(row.token), Some(row.scryfall_id), "{}", row.name);
        }
        assert_eq!(picture(u16::MAX), None);
    }
}
