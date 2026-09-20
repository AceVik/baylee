//! Miren, the Moaning Well — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness.
//! Set: SOK #163 — Saviors of Kamigawa | Scryfall ID: 53d414f0-15ae-446b-a8d7-56c1b502740c | Oracle ID: 03fe19bb-8e22-4030-8299-2ddd2d5a7eb2
// PARTIAL — {T}: Add {C} is the whole card that can be said. The sacrifice
// ability's cost is payable (CostPart::Sacrifice), but the life it grants
// reads a number nothing in the DSL can read.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "{3}, {T}, Sacrifice a creature: You gain life equal to the
// sacrificed creature's toughness." — no `Amount` variant reads the toughness
// of the permanent that paid a *cost*: Amount::TargetPower reads the power of
// the first *target*, and the sacrificed creature is neither targeted nor
// asked about its power. The ability is therefore left off the card rather
// than shipped as a sacrifice that gains the wrong amount of life.

card!(
    index = index::MIREN_THE_MOANING_WELL,
    oracle_id = "03fe19bb-8e22-4030-8299-2ddd2d5a7eb2",
    scryfall_id = "53d414f0-15ae-446b-a8d7-56c1b502740c",
    faces = &[face!(
        name = "Miren, the Moaning Well",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    coverage = Coverage::Partial(
        "the sacrifice ability gains life equal to the sacrificed creature's toughness, and no Amount variant can read that"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
