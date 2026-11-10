//! Mirrex — (no cost) — Land — Sphere
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Activate only if this land entered this turn.
//! Oracle: {3}, {T}: Create a 1/1 colorless Phyrexian Mite artifact creature token with toxic 1 and "This token can't block." (Players dealt combat damage by it also get a poison counter.)
//! Set: ONE #254 — Phyrexia: All Will Be One | Scryfall ID: 54a702cd-ca49-4570-b47e-8b090452a3c3 | Oracle ID: 5502741a-e3b9-454e-8121-4360a6db6750

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MIRREX,
    oracle_id = "5502741a-e3b9-454e-8121-4360a6db6750",
    scryfall_id = "54a702cd-ca49-4570-b47e-8b090452a3c3",
    faces = &[face!(
        name = "Mirrex",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::SPHERE],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            Cost::TAP,
            &[Effect::mana_of_any_color()],
            condition = Some(Condition::SourceMatches(&Filter::EnteredThisTurn))
        ),
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::CreateToken {
                token: &crate::tokens::PHYREXIAN_MITE_1_1
            }]
        ),
    ],
);
