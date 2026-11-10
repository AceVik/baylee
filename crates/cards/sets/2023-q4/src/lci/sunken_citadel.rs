//! Sunken Citadel — (no cost) — Land — Cave
//! Oracle: This land enters tapped. As it enters, choose a color.
//! Oracle: {T}: Add one mana of the chosen color.
//! Oracle: {T}: Add two mana of the chosen color. Spend this mana only to activate abilities of land sources.
//! Set: LCI #285 — The Lost Caverns of Ixalan | Scryfall ID: 3e1c9b1a-e306-47bb-9f68-2083660319c0 | Oracle ID: 508189e1-9cef-4f9c-8ff1-078c99a0f603

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUNKEN_CITADEL,
    oracle_id = "508189e1-9cef-4f9c-8ff1-078c99a0f603",
    scryfall_id = "3e1c9b1a-e306-47bb-9f68-2083660319c0",
    faces = &[face!(
        name = "Sunken Citadel",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
        enter_modifiers = &[EnterModifier::ChooseColor, EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_chosen()]),
        mana_ability!(&[Effect::AddMana {
            source: ManaSource::Chosen,
            amount: Amount::Fixed(2),
            distribution: baylee_cards_dsl::ManaDistribution::OneColor,
            restriction: None,
        }
        .restricted_for(&Filter::LAND, SpendRider::None, ManaUse::Abilities)]),
    ],
);
