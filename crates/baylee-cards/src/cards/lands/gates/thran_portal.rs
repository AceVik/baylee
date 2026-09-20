//! Thran Portal — (no cost) — Land — Gate
//! Oracle: This land enters tapped unless you control two or fewer other lands.
//! Oracle: As this land enters, choose a basic land type.
//! Oracle: This land is the chosen type in addition to its other types.
//! Oracle: Mana abilities of this land cost an additional 1 life to activate.
//! Set: DMU #259 — Dominaria United | Scryfall ID: ef074a2e-a387-4af8-a180-74b145d93992 | Oracle ID: 926ce6a2-7bdd-4380-ac65-bc902ba0c284
// PARTIAL — the fastland entry clause ("two or fewer other lands") is built; the rest of the card is not expressible.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: "As this land enters, choose a basic land type." — EnterModifier::ChooseSubtype offers creature types, the only list the engine carries.
// NOT SUPPORTED: "This land is the chosen type in addition to its other types." — Modifier::AddSubtype takes a fixed subtype and nothing applies a chosen one, so the land gains no basic land type (and so taps for nothing).
// NOT SUPPORTED: "Mana abilities of this land cost an additional 1 life to activate." — no Modifier states a cost, and CostPart::PayLife would sit on a mana ability this card does not print.

card!(
    index = index::THRAN_PORTAL,
    oracle_id = "926ce6a2-7bdd-4380-ac65-bc902ba0c284",
    scryfall_id = "ef074a2e-a387-4af8-a180-74b145d93992",
    faces = &[face!(
        name = "Thran Portal",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::GATE],
        enter_modifiers = &[EnterModifier::TappedUnlessAtMost {
            filter: &Filter::YOUR_LAND,
            at_most: 2,
        }],
    ),],
    coverage = Coverage::Partial(
        "choose a basic land type, the type it grants, and the +1 life tax on its mana abilities are not expressible",
    ),
);
