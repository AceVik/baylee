//! Havengul Laboratory // Havengul Mystery — (no cost) — Legendary Land // Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {4}, {T}: Investigate. (Create a colorless Clue artifact token with "{2}, Sacrifice this artifact: Draw a card.")
//! Oracle: At the beginning of your end step, if you sacrificed three or more Clues this turn, transform Havengul Laboratory.
//! Oracle: When this land transforms into Havengul Mystery, return target creature card from your graveyard to the battlefield.
//! Oracle: When the creature put onto the battlefield with Havengul Mystery leaves the battlefield, transform Havengul Mystery.
//! Oracle: {T}, Pay 1 life: Add {B}.
//! Set: SLX #9 — Universes Within | Scryfall ID: 823b019e-10c0-4712-8167-d4f37a71e782 | Oracle ID: e71ac446-02a4-4468-8d29-f28b21617665
//! Face: Havengul Laboratory —  — Legendary Land
//! Face: Havengul Mystery —  — Legendary Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card! {
    index: 609,
    oracle_id: "e71ac446-02a4-4468-8d29-f28b21617665",
    scryfall_id: "823b019e-10c0-4712-8167-d4f37a71e782",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Havengul Laboratory",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    face! {
        name: "Havengul Mystery",
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
