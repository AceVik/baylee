//! Scroll Rack — {2} — Artifact
//! Oracle: {1}, {T}: Exile any number of cards from your hand face down. Put that many cards from the top of your library into your hand. Then look at the exiled cards and put them on top of your library in any order.
//! Set: CMR #337 — Commander Legends | Scryfall ID: dfaaa58d-89bb-4cb3-96a6-b480e6f6954e | Oracle ID: 648bd03d-247e-4d6f-87a9-12a67688cbf5
// PARTIAL: taps with {1} to draw a card and put a card from your hand on top of your library; exiling any number of cards from your hand face down, putting that many cards from the top of your library into your hand, and then putting the exiled cards on top of your library in any order is not in Effect.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCROLL_RACK,
    oracle_id = "648bd03d-247e-4d6f-87a9-12a67688cbf5",
    scryfall_id = "dfaaa58d-89bb-4cb3-96a6-b480e6f6954e",
    coverage = Coverage::Partial(
        "taps with {1} to draw a card and put a card from your hand on top of your library; exiling any number of cards from your hand face down, putting that many cards from the top of your library into your hand, and then putting the exiled cards on top of your library in any order is not in Effect",
    ),
    faces = &[face!(
        name = "Scroll Rack",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{1}", TapSelf),
        &[Effect::draw(1), Effect::PutFromHandOnTop { count: 1 }],
    )],
);
