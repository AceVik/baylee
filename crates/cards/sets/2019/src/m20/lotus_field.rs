//! Lotus Field — (no cost) — Land
//! Oracle: Hexproof
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, sacrifice two lands.
//! Oracle: {T}: Add three mana of any one color.
//! Set: SOC #385 — Secrets of Strixhaven Commander | Scryfall ID: ee5ac47c-e8b6-400e-b91e-a7cd0f952cf1 | Oracle ID: 134d5b82-7940-4b33-a922-7f9d1f403e50
// IMPLEMENTED — hexproof, enters tapped, an ETB that sacrifices two lands
// (one `SacrificeFilter` per land, in the order the card prints them), and
// {T} for three mana of one colour chosen as the ability resolves.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LOTUS_FIELD,
    oracle_id = "134d5b82-7940-4b33-a922-7f9d1f403e50",
    scryfall_id = "ee5ac47c-e8b6-400e-b91e-a7cd0f952cf1",
    faces = &[face!(
        name = "Lotus Field",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    keywords = KeywordSet::HEXPROOF,
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[
                Effect::SacrificeFilter {
                    who: PlayerRel::You,
                    filter: &Filter::LAND,
                },
                Effect::SacrificeFilter {
                    who: PlayerRel::You,
                    filter: &Filter::LAND,
                },
            ]
        ),
        mana_ability!(&[Effect::mana_choice_dynamic(
            ALL_MANA_COLORS,
            Amount::Fixed(3)
        )]),
    ],
);
