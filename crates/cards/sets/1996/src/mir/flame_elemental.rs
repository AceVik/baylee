//! Flame Elemental — {2}{R}{R} — Creature — Elemental
//! Oracle: {R}, {T}, Sacrifice this creature: It deals damage equal to its power to target creature.
//! Set: MIR #175 — Mirage | Scryfall ID: 498e813d-b0b3-4040-b87a-4fa2be681ec5 | Oracle ID: 975226a4-2d03-4993-9337-cfd6595011bd
// IMPLEMENTED — {R}, {T}, Sacrifice this creature: deals damage equal to its power to target creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLAME_ELEMENTAL,
    oracle_id = "975226a4-2d03-4993-9337-cfd6595011bd",
    scryfall_id = "498e813d-b0b3-4040-b87a-4fa2be681ec5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Flame Elemental",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{R}", TapSelf, SacrificeSelf),
        &[Effect::DealDamage {
            amount: Amount::SourcePower,
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
