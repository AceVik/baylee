//! Ifnir Deadlands — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add {B}.
//! Oracle: {2}{B}{B}, {T}, Sacrifice a Desert: Put two -1/-1 counters on target creature an opponent controls. Activate only as a sorcery.
//! Set: ECC #153 — Lorwyn Eclipsed Commander | Scryfall ID: 902e260d-71ba-4342-9794-fefe2b531c00 | Oracle ID: af698bd5-5f56-4d2a-9f02-8c3e781210cd
// IMPLEMENTED — {C} and {B} (for 1 life) mana abilities, plus Desert-sacrifice putting two -1/-1 counters on target opponent creature at sorcery speed.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IFNIR_DEADLANDS,
    oracle_id = "af698bd5-5f56-4d2a-9f02-8c3e781210cd",
    scryfall_id = "902e260d-71ba-4342-9794-fefe2b531c00",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Ifnir Deadlands",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!(TapSelf, PayLife(1)),
            &[Effect::mana(ManaColor::Black, 1)],
        ),
        activated!(
            cost!(
                "{2}{B}{B}",
                TapSelf,
                Sacrifice(&Filter::HasSubtype(subtypes::land::DESERT)),
            ),
            &[Effect::AddCounter {
                kind: CounterKind::M1M1,
                amount: Amount::Fixed(2),
            }],
            target = Some(TargetSpec::Object(&Filter::OPPONENT_CREATURE)),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
