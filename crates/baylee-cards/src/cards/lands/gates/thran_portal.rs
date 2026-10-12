//! Thran Portal — (no cost) — Land — Gate
//! Oracle: This land enters tapped unless you control two or fewer other lands.
//! Oracle: As this land enters, choose a basic land type.
//! Oracle: This land is the chosen type in addition to its other types.
//! Oracle: Mana abilities of this land cost an additional 1 life to activate.
//! Set: DMU #259 — Dominaria United | Scryfall ID: ef074a2e-a387-4af8-a180-74b145d93992 | Oracle ID: 926ce6a2-7bdd-4380-ac65-bc902ba0c284
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THRAN_PORTAL,
    oracle_id = "926ce6a2-7bdd-4380-ac65-bc902ba0c284",
    scryfall_id = "ef074a2e-a387-4af8-a180-74b145d93992",
    faces = &[face!(
        name = "Thran Portal",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::GATE],
        enter_modifiers = &[
            EnterModifier::TappedUnlessAtMost {
                filter: &Filter::YOUR_LAND,
                at_most: 2
            },
            EnterModifier::ChooseBasicLandType,
        ],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::AddChosenBasicLandType),
        static_ability!(Filter::This, Modifier::ManaAbilitiesCostLife(1)),
    ],
);
