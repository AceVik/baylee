//! Untaidake, the Cloud Keeper — (no cost) — Legendary Land
//! Oracle: Untaidake enters tapped.
//! Oracle: {T}, Pay 2 life: Add {C}{C}. Spend this mana only to cast legendary spells.
//! Set: CHK #285 — Champions of Kamigawa | Scryfall ID: 3c571f66-7a00-4cb0-9da9-8271083f49d3 | Oracle ID: 362f25a6-01ff-4c53-be52-c6346a9b0065
// IMPLEMENTED — enters tapped; {T}, pay 2 life for {C}{C}, spendable only on
// legendary spells (`Effect::restricted`).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNTAIDAKE_THE_CLOUD_KEEPER,
    oracle_id = "362f25a6-01ff-4c53-be52-c6346a9b0065",
    scryfall_id = "3c571f66-7a00-4cb0-9da9-8271083f49d3",
    faces = &[face!(
        name = "Untaidake, the Cloud Keeper",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[mana_ability!(
        cost!(TapSelf, PayLife(2)),
        &[Effect::mana(ManaColor::Colorless, 2).restricted(
            &Filter::HasSupertype(SupertypeSet::LEGENDARY),
            SpendRider::None
        )]
    )],
);
