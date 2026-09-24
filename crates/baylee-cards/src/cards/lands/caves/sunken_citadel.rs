//! Sunken Citadel — (no cost) — Land — Cave
//! Oracle: This land enters tapped. As it enters, choose a color.
//! Oracle: {T}: Add one mana of the chosen color.
//! Oracle: {T}: Add two mana of the chosen color. Spend this mana only to activate abilities of land sources.
//! Set: LCI #285 — The Lost Caverns of Ixalan | Scryfall ID: 3e1c9b1a-e306-47bb-9f68-2083660319c0 | Oracle ID: 508189e1-9cef-4f9c-8ff1-078c99a0f603
// PARTIAL — enters tapped, chooses a color as it enters, and taps for one
// mana of that color; the two-mana line is off, because its spend
// restriction is not expressible (see NOT SUPPORTED below).

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
        "the two-mana ability is left off: its \"spend this mana only to activate abilities of land sources\" cannot be stated, because a ManaRestriction admits spells only and restricted mana never pays for an activated ability",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana_chosen()]),
        // NOT SUPPORTED: "{T}: Add two mana of the chosen color. Spend this
        // mana only to activate abilities of land sources." A ManaRestriction
        // is asked only of a spell being cast, and restricted mana never pays
        // for an activated ability, so any restriction written here would
        // make the mana unspendable on the abilities it is for. Written
        // without one, the land made two mana for anything, more than printed.
    ],
);
