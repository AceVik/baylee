//! Freyalise Supplicant — {1}{G} — Creature — Human Cleric
//! Oracle: {T}, Sacrifice a red or white creature: This creature deals damage to any target equal to half the sacrificed creature's power, rounded down.
//! Set: ICE #239 — Ice Age | Scryfall ID: 5b1e718a-882a-4bdc-9d62-4dda88da0ba0 | Oracle ID: c28a7641-49e3-43b1-a366-8d6e9c8dab9c

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const RED_OR_WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::Red, Color::White])),
]);

card!(
    index = index::FREYALISE_SUPPLICANT,
    oracle_id = "c28a7641-49e3-43b1-a366-8d6e9c8dab9c",
    scryfall_id = "5b1e718a-882a-4bdc-9d62-4dda88da0ba0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Freyalise Supplicant",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost {
            mana: ManaCost::ZERO,
            parts: &[
                CostPart::TapSelf,
                CostPart::Sacrifice(&RED_OR_WHITE_CREATURE)
            ],
        },
        &[Effect::DealDamage {
            amount: Amount::Half {
                base: &Amount::SacrificedPower,
                round_up: false,
            },
            target: TargetSpec::AnyTarget,
        }],
        target = Some(TargetSpec::AnyTarget),
    )],
);
