//! Gamble — {R} — Sorcery
//! Oracle: Search your library for a card, put that card into your hand, discard a card at random, then shuffle.
//! Set: DMR #121 — Dominaria Remastered | Scryfall ID: 8e37fae5-ddd0-4e16-8581-71579f89d9c5 | Oracle ID: a54f0869-94c8-42af-9080-166efb9486a4
// PARTIAL — the tutor and the shuffle a search carries are built; the random
// discard is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GAMBLE,
    oracle_id = "a54f0869-94c8-42af-9080-166efb9486a4",
    scryfall_id = "8e37fae5-ddd0-4e16-8581-71579f89d9c5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Gamble",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial("discard a card at random"),
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &Filter::Any,
        finds: &[Find::HAND],
        optional: false,
    }])],
);

// NOT SUPPORTED: "discard a card at random" — no Effect discards from the
// caster's own hand by chance. DiscardForPlayers asks the player which card,
// which is the opposite of what the printed clause does; the search half
// (and the shuffle, which a printed search derives) is implemented above.
