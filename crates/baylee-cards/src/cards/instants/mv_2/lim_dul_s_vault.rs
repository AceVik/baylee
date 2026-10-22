//! Lim-Dûl's Vault — {U}{B} — Instant
//! Oracle: Look at the top five cards of your library. As many times as you choose, you may pay 1 life, put those cards on the bottom of your library in any order, then look at the top five cards of your library. Then shuffle and put the last cards you looked at this way on top in any order.
//! Set: C13 #197 — Commander 2013 | Scryfall ID: cec89c38-0b72-44b0-ac6c-7eb9503e1256 | Oracle ID: 3f8e7a45-4c6e-4ee6-93d0-b7de9715ec97
// NOT SUPPORTED: repeatedly paying 1 life to put the top five cards on the bottom of your library, look at the next five, and shuffle the rest of your library before putting the last five on top has no DSL representation.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIM_DUL_S_VAULT,
    oracle_id = "3f8e7a45-4c6e-4ee6-93d0-b7de9715ec97",
    scryfall_id = "cec89c38-0b72-44b0-ac6c-7eb9503e1256",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Partial(
        "The repeatable loop to pay 1 life, put the top five cards on the bottom of your library, look at the next five, and shuffle the rest of your library before putting the last five on top has no DSL representation; in play, Lim-Dûl's Vault looks at the top five cards of your library and puts them back on top in any order.",
    ),
    faces = &[face!(
        name = "Lim-Dûl's Vault",
        mana_cost = mana!("{U}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::ReorderTopLibrary { count: 5 }])],
);
