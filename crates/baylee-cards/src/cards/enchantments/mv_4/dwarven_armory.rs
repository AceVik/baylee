//! Dwarven Armory — {2}{R}{R} — Enchantment
//! Oracle: {2}, Sacrifice a land: Put a +2/+2 counter on target creature. Activate only during any upkeep step.
//! Set: ICE #182 — Ice Age | Scryfall ID: 7d14a430-6e08-40cf-970a-cae84bba6ef7 | Oracle ID: aba53a54-8666-430d-899e-45ad9f783a4a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DWARVEN_ARMORY,
    oracle_id = "aba53a54-8666-430d-899e-45ad9f783a4a",
    scryfall_id = "7d14a430-6e08-40cf-970a-cae84bba6ef7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dwarven Armory",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!("{2}", Sacrifice(&Filter::LAND)),
        &[Effect::AddCounter {
            kind: CounterKind::Plus {
                power: 2,
                toughness: 2,
            },
            amount: Amount::Fixed(1),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
        condition = Some(Condition::DuringStep(StepKind::Upkeep)),
    )],
);
