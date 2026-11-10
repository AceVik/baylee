//! Miren, the Moaning Well — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
//! Set: SOK #163 — Saviors of Kamigawa | Scryfall ID: 53d414f0-15ae-446b-a8d7-56c1b502740c | Oracle ID: 03fe19bb-8e22-4030-8299-2ddd2d5a7eb2
// IMPLEMENTED — {T}: Add {C}, and the sacrifice ability gains life equal to
// the sacrificed creature's toughness (Amount::SacrificedToughness).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIREN_THE_MOANING_WELL,
    oracle_id = "03fe19bb-8e22-4030-8299-2ddd2d5a7eb2",
    scryfall_id = "53d414f0-15ae-446b-a8d7-56c1b502740c",
    faces = &[face!(
        name = "Miren, the Moaning Well",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{3}", TapSelf, Sacrifice(&Filter::CREATURE)),
            &[Effect::GainLife {
                amount: Amount::SacrificedToughness
            }]
        ),
    ],
);
