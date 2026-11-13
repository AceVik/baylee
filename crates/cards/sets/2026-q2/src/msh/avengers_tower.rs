//! Avengers Tower — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast a Hero spell or to activate an ability of a Hero source.
//! Oracle: {4}, {T}: Look at the top three cards of your library. You may reveal a Hero card from among them and put it into your hand. Put the rest on the bottom of your library in any order.
//! Set: MSH #260 — Marvel Super Heroes | Scryfall ID: 88f0d9c9-8a1f-4b5a-b6f9-821ddd658d27 | Oracle ID: c5fc8e7c-a87e-4586-a13c-d30e0a3aafbf
// Batch 46: every printed ability is implemented; package validation is pending.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::AVENGERS_TOWER,
    oracle_id = "c5fc8e7c-a87e-4586-a13c-d30e0a3aafbf",
    scryfall_id = "88f0d9c9-8a1f-4b5a-b6f9-821ddd658d27",
    faces = &[face!(name = "Avengers Tower", types = TypeSet::LAND,)],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color().restricted_for(
            &Filter::HasSubtype(creature::HERO),
            SpendRider::None,
            ManaUse::SpellsOrAbilities,
        )]),
        activated!(
            cost!("{4}", TapSelf),
            &[Effect::LookAtTopFind {
                found: Find::HAND,
                count: 3,
                filter: &Filter::HasSubtype(creature::HERO),
                random: false,
            }]
        ),
    ],
);
