//! Sunken Citadel — (no cost) — Land — Cave
//! Oracle: This land enters tapped. As it enters, choose a color.
//! Oracle: {T}: Add one mana of the chosen color.
//! Oracle: {T}: Add two mana of the chosen color. Spend this mana only to activate abilities of land sources.
//! Set: LCI #285 — The Lost Caverns of Ixalan | Scryfall ID: 3e1c9b1a-e306-47bb-9f68-2083660319c0 | Oracle ID: 508189e1-9cef-4f9c-8ff1-078c99a0f603
// PARTIAL — enters tapped, chooses a color as it enters, and taps for one or
// two mana of that color; the second mana line's spend restriction is not
// expressible (see NOT SUPPORTED below).

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
    coverage = Coverage::Partial(
        "the second mana ability's \"spend this mana only to activate abilities of land sources\" cannot be stated: ManaRestriction filters spells and pool mana has no provenance",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana_chosen()]),
        // NOT SUPPORTED: "Spend this mana only to activate abilities of land
        // sources." ManaRestriction says which *spells* restricted mana may be
        // spent on, and no filter reaches an activation; the rider itself is
        // unenforced because pool mana carries no provenance (M3+).
        mana_ability!(&[Effect::AddMana {
            source: ManaSource::Chosen,
            amount: Amount::Fixed(2),
            combination: false,
            restriction: None,
        }]),
    ],
);
