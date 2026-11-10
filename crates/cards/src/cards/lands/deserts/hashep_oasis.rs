//! Hashep Oasis — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add {G}.
//! Oracle: {1}{G}{G}, {T}, Sacrifice a Desert: Target creature gets +3/+3 until end of turn. Activate only as a sorcery.
//! Set: OTC #299 — Outlaws of Thunder Junction Commander | Scryfall ID: d18d5af2-ca2c-4a3a-9b67-e953b24b0718 | Oracle ID: eab70fff-6a9f-4f9f-89a2-b6910c199e46
// IMPLEMENTED — {C} and {G} (for 1 life) mana abilities, plus Desert-sacrifice +3/+3 pump target at sorcery speed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HASHEP_OASIS,
    oracle_id = "eab70fff-6a9f-4f9f-89a2-b6910c199e46",
    scryfall_id = "d18d5af2-ca2c-4a3a-9b67-e953b24b0718",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Hashep Oasis",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!(TapSelf, PayLife(1)),
            &[Effect::mana(ManaColor::Green, 1)],
        ),
        activated!(
            cost!(
                "{1}{G}{G}",
                TapSelf,
                Sacrifice(&Filter::HasSubtype(subtypes::land::DESERT)),
            ),
            &[Effect::PumpTarget {
                power: Amount::Fixed(3),
                toughness: Amount::Fixed(3),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
