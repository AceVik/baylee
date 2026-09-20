//! Library of Alexandria — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Draw a card. Activate only if you have exactly seven cards in hand.
//! Set: VMA #303 — Vintage Masters | Scryfall ID: e5145f31-a4ac-44ef-8f85-e4d95f2c9ff5 | Oracle ID: 2111588d-9af5-4a33-989e-b074d83f0463
//! PARTIAL — the {T} for {C} mana ability is whole; the draw ability is
//! dropped rather than offered ungated, because `Condition` cannot count a
//! hand.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIBRARY_OF_ALEXANDRIA,
    oracle_id = "2111588d-9af5-4a33-989e-b074d83f0463",
    scryfall_id = "e5145f31-a4ac-44ef-8f85-e4d95f2c9ff5",
    faces = &[face!(name = "Library of Alexandria", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "\"{T}: Draw a card\" is gated on having exactly seven cards in hand, \
         and no Condition variant counts a hand (ControlCount counts \
         permanents, CountersOnSelf counts counters, SourceMatches reads the \
         source); the ability is dropped rather than offered with its gate \
         missing"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{T}: Draw a card. Activate only if you have exactly
        // seven cards in hand." — the activation condition is a hand size,
        // and an ungated {T}: Draw a card is a different card.
    ],
);
