//! Recall — {X}{X}{U} — Sorcery
//! Oracle: Discard X cards, then return a card from your graveyard to your hand for each card discarded this way. Exile Recall.
//! Set: ME3 #46 — Masters Edition III | Scryfall ID: dd82d1f8-12bb-4b71-9a25-76a9dcb68345 | Oracle ID: 4158f397-4ee4-4f53-90f2-3b64bfe4f9b8
// PARTIAL — "Exile Recall" is written; the discard/return sentence is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RECALL,
    oracle_id = "4158f397-4ee4-4f53-90f2-3b64bfe4f9b8",
    scryfall_id = "dd82d1f8-12bb-4b71-9a25-76a9dcb68345",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "no effect discards an announced X cards of the player's choice, and \
         no Amount reads the cards a resolution discarded to size the return"
    ),
    faces = &[face!(
        name = "Recall",
        mana_cost = mana!("{X}{X}{U}"),
        types = TypeSet::SORCERY,
    ),],
    // NOT SUPPORTED: "Discard X cards, then return a card from your graveyard
    // to your hand for each card discarded this way." —
    // `Effect::DiscardForPlayers` takes a fixed `u8`, `Effect::DiscardRandom`
    // discards at random, and `Effect::DiscardUpToThenDraw` is capped by a
    // constant and draws instead. Nothing remembers how many cards a
    // resolution discarded, so the return cannot be sized.
    abilities = &[spell!(&[Effect::ExileSource])],
);
