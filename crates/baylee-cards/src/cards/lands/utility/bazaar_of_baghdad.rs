//! Bazaar of Baghdad — (no cost) — Land
//! Oracle: {T}: Draw two cards, then discard three cards.
//! Set: VMA #294 — Vintage Masters | Scryfall ID: c88acaa8-ad4d-4321-a6f6-9361916e5b5e | Oracle ID: 54022a10-c9f0-458d-a0ed-228843cd9a40
// IMPLEMENTED — {T}: draw two cards, then discard three.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BAZAAR_OF_BAGHDAD,
    oracle_id = "54022a10-c9f0-458d-a0ed-228843cd9a40",
    scryfall_id = "c88acaa8-ad4d-4321-a6f6-9361916e5b5e",
    faces = &[face!(name = "Bazaar of Baghdad", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        Cost::TAP,
        &[
            Effect::draw(2),
            Effect::DiscardForPlayers {
                who: PlayerRel::You,
                count: 3
            },
        ]
    )],
);
