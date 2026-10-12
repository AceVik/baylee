//! Underdome — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to pay Un-costs.
//! Set: UND #86 — Unsanctioned | Scryfall ID: d5c362ea-16e2-4c13-bed2-735c8c09d975 | Oracle ID: 8375aaaa-edc2-4a0c-98f6-af07d61ebd0a

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNDERDOME,
    oracle_id = "8375aaaa-edc2-4a0c-98f6-af07d61ebd0a",
    scryfall_id = "d5c362ea-16e2-4c13-bed2-735c8c09d975",
    faces = &[face!(name = "Underdome", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color().restricted_for(
            &Filter::UnCard,
            SpendRider::None,
            ManaUse::CostsFromMatchingSource
        )])
    ],
);
