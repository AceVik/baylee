//! Samite Alchemist — {3}{W} — Creature — Human Cleric
//! Oracle: {W}{W}, {T}: Prevent the next 4 damage that would be dealt this turn to target creature you control. Tap that creature. It doesn't untap during your next untap step.
//! Set: HML #13a — Homelands | Scryfall ID: 68224871-d4c2-4b43-9ab8-c0685588b035 | Oracle ID: 120b565a-e22b-4d5d-9841-b11750b4ee49
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAMITE_ALCHEMIST,
    oracle_id = "120b565a-e22b-4d5d-9841-b11750b4ee49",
    scryfall_id = "68224871-d4c2-4b43-9ab8-c0685588b035",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Samite Alchemist",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(0),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{W}{W}", TapSelf),
        &[
            Effect::PreventNextDamage {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
                amount: Amount::Fixed(4),
            },
            Effect::TapTarget,
            Effect::continuous(
                &Filter::This,
                Modifier::DoesNotUntap,
                Duration::UntilYourNextUntapStep,
            ),
        ],
        target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE))
    )],
);
