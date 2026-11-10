//! Fungus Elemental — {3}{G} — Creature — Fungus Elemental
//! Oracle: {G}, Sacrifice a Forest: Put a +2/+2 counter on this creature. Activate only if this creature entered this turn.
//! Set: WTH #128 — Weatherlight | Scryfall ID: 4336bfd1-27a4-414d-b6fe-f186a0563dc0 | Oracle ID: 3d3c5dd5-ad4e-48ca-92b1-91e1f4ad5c3b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static FOREST_FILTER: Filter = Filter::HasSubtype(subtypes::land::FOREST);

card!(
    index = index::FUNGUS_ELEMENTAL,
    oracle_id = "3d3c5dd5-ad4e-48ca-92b1-91e1f4ad5c3b",
    scryfall_id = "4336bfd1-27a4-414d-b6fe-f186a0563dc0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fungus Elemental",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FUNGUS, subtypes::creature::ELEMENTAL],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{G}", Sacrifice(&FOREST_FILTER)),
        &[Effect::AddCounter {
            kind: CounterKind::Plus {
                power: 2,
                toughness: 2,
            },
            amount: Amount::Fixed(1),
        }],
        condition = Some(Condition::SourceMatches(&Filter::EnteredThisTurn)),
    )],
);
