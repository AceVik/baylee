//! Shefet Dunes — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add {W}.
//! Oracle: {2}{W}{W}, {T}, Sacrifice a Desert: Creatures you control get +1/+1 until end of turn. Activate only as a sorcery.
//! Set: OTC #318 — Outlaws of Thunder Junction Commander | Scryfall ID: c9b0a526-73b0-4501-80a3-f16dbef9cdfd | Oracle ID: 8305715e-f711-47d6-8efe-d0efe4ced418
// IMPLEMENTED — {C} and {W} (for 1 life) mana abilities, plus Desert-sacrifice +1/+1 team pump at sorcery speed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHEFET_DUNES,
    oracle_id = "8305715e-f711-47d6-8efe-d0efe4ced418",
    scryfall_id = "c9b0a526-73b0-4501-80a3-f16dbef9cdfd",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Shefet Dunes",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!(TapSelf, PayLife(1)),
            &[Effect::mana(ManaColor::White, 1)],
        ),
        activated!(
            cost!(
                "{2}{W}{W}",
                TapSelf,
                Sacrifice(&Filter::HasSubtype(subtypes::land::DESERT)),
            ),
            &[Effect::PumpFilter {
                filter: &Filter::YOUR_CREATURE,
                controlled_by: None,
                power: Amount::Fixed(1),
                toughness: Amount::Fixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
