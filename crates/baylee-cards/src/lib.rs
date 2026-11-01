//! baylee-cards — the compiled card registry.
//!
//! One file per card in [`cards`], dense lookup tables in [`generated`]
//! (both produced by `cargo xtask codegen`).

#![warn(missing_docs)]

use baylee_cards_dsl::CardDef;
use baylee_core::ids::CardIndex;

/// Deterministic CardDsl AST encoder and semantic summary vectors.
pub mod carddsl;
/// Generated: one module per card.
pub mod cards;
/// Synergistic, curve-balanced, pip-accurate deck generator and coverage corpus builder.
pub mod deckgen;
/// Deck parsing and name resolution against the registry (acceptance
/// deck format, `"N Card Name"` lines, preset assembly).
pub mod decks;
/// What the lobby's deck list says about a deck: copies, colours,
/// commanders (#254).
pub mod digest;
/// Filters shared by more than one card file.
pub mod filters;
/// Deck shapes: size, copy limits, leaders and colour identity.
pub mod formats;
/// Generated: registry tables.
pub mod generated;
/// Generated: which printed sentence each ability came from.
pub mod generated_lines;
/// Generated: the name table — which card a printed English name is.
pub mod generated_names;
/// Generated: every card's English Oracle text, per face.
pub mod generated_oracle;
/// Generated: which cards have a back, and which are double-faced.
pub mod generated_sides;
/// Generated: the token ledger — which id every token there is was assigned.
pub mod generated_tokens;
/// Which printed sentence an ability came from (the reader of
/// [`generated_lines`]).
pub mod lines;
/// Pool-wide lints over the card data (tests only).
#[cfg(test)]
mod lints;
/// The L5 mutation switch, `BAYLEE_MUTATE` (tests and the `mutate` feature
/// only; `docs/verification-hooks.md`).
#[cfg(any(test, feature = "mutate"))]
pub mod mutate;
/// The English Oracle text of a card's face (the reader of
/// [`generated_oracle`]).
pub mod oracle;
/// The registry as deck-builder rows — what a deck may be built from.
pub mod pool;
/// Verified physical printing facts and per-copy deck choices.
pub mod print_rules;
/// Which kinds of mana a card's printed mana abilities name (`/pool`'s
/// `produces`).
pub mod produces;
/// Whether a card has a back to show, and whether it is double-faced.
pub mod sides;
/// Central named token definitions (referenced by card files).
pub mod tokens;

pub use baylee_cards_dsl as dsl;

/// Looks up a card definition by Scryfall oracle id.
#[must_use]
pub fn by_oracle_id(oracle_id: &str) -> Option<&'static CardDef> {
    generated::by_oracle_id(oracle_id).map(mutated)
}

/// Looks up a card definition by its dense runtime index.
#[must_use]
pub fn by_index(index: CardIndex) -> Option<&'static CardDef> {
    generated::by_index(index).map(mutated)
}

// The registry's three doors hand out a mutant in the card's place while
// `BAYLEE_MUTATE` names one (`mutate`), and the card itself everywhere else.
#[cfg(any(test, feature = "mutate"))]
use mutate::apply as mutated;

#[cfg(not(any(test, feature = "mutate")))]
#[inline]
const fn mutated(def: &'static CardDef) -> &'static CardDef {
    def
}

/// Number of registered cards.
#[must_use]
pub fn count() -> usize {
    generated::ALL.len()
}

/// Every card in the registry, in index order.
///
/// The pool is what a deck builder can offer, so something has to be able to
/// walk it. Index order rather than table order: the order is what a player
/// sees when nothing else sorts the list, and a `HashMap`'s order is not an
/// order.
pub fn all() -> impl Iterator<Item = &'static CardDef> {
    generated::BY_INDEX
        .iter()
        .filter_map(|slot| *slot)
        .map(mutated)
}

/// Hash of the whole pool (client cache invalidation / gateway handshake).
#[must_use]
pub fn pool_hash() -> u64 {
    generated::POOL_HASH
}

#[cfg(test)]
mod tests;
