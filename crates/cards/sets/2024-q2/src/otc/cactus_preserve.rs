//! Cactus Preserve — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add one mana of any type that a land you control could produce.
//! Oracle: {3}: Until end of turn, this land becomes an X/X green Plant creature with reach, where X is the greatest mana value among your commanders. It's still a land.
//! Set: OTC #40 — Outlaws of Thunder Junction Commander | Scryfall ID: ad9d426f-5870-42bb-a589-9218f7e35d62 | Oracle ID: 8da29533-f389-4bc2-ab9b-b469f893a362

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CACTUS_PRESERVE,
    oracle_id = "8da29533-f389-4bc2-ab9b-b469f893a362",
    scryfall_id = "ad9d426f-5870-42bb-a589-9218f7e35d62",
    faces = &[face!(
        name = "Cactus Preserve",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_land_type(true)]),
        activated!(
            cost!("{3}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::PLANT),
                    Duration::UntilEndOfTurn
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetColor(ColorSet::of(Color::Green)),
                    Duration::UntilEndOfTurn
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::REACH),
                    Duration::UntilEndOfTurn
                ),
                Effect::SetPTFilter {
                    filter: &Filter::This,
                    power: Amount::GreatestCommanderManaValue,
                    toughness: Amount::GreatestCommanderManaValue,
                    duration: Duration::UntilEndOfTurn
                },
            ]
        )
    ],
);
