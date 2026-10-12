//! Brotherhood Headquarters — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any color. Spend this mana only to cast an Assassin spell or a spell that has freerunning, or to activate an ability of an Assassin source.
//! Set: ACR #80 — Assassin's Creed | Scryfall ID: 535f43c4-8926-4981-967d-f681f98e07d9 | Oracle ID: 0d3a06d5-5bb9-4733-a55b-9e2c75de6b6e
// Batch 46: every printed ability is implemented; package validation is pending.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::BROTHERHOOD_HEADQUARTERS,
    oracle_id = "0d3a06d5-5bb9-4733-a55b-9e2c75de6b6e",
    scryfall_id = "535f43c4-8926-4981-967d-f681f98e07d9",
    faces = &[face!(
        name = "Brotherhood Headquarters",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(&[Effect::mana_of_any_color()
            .restricted_for(
                &Filter::Or(&[
                    Filter::HasSubtype(creature::ASSASSIN),
                    Filter::HasKeyword(KeywordSet::FREERUNNING),
                ]),
                SpendRider::None,
                ManaUse::SpellsOrAbilities,
            )
            .for_abilities_matching(&Filter::HasSubtype(creature::ASSASSIN))]),
    ],
);
