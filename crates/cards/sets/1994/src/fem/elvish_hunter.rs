//! Elvish Hunter — {1}{G} — Creature — Elf Archer
//! Oracle: {1}{G}, {T}: Target creature doesn't untap during its controller's next untap step.
//! Set: ME2 #157 — Masters Edition II | Scryfall ID: baf265bf-0f83-4073-87c2-f0b9ef268592 | Oracle ID: 49b351d8-8b63-4f9e-a323-344ed35fdfff
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELVISH_HUNTER,
    oracle_id = "49b351d8-8b63-4f9e-a323-344ed35fdfff",
    scryfall_id = "baf265bf-0f83-4073-87c2-f0b9ef268592",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Elvish Hunter",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::ARCHER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{G}", TapSelf),
        &[Effect::SuppressUntap {
            target: TargetSpec::Object(&Filter::CREATURE),
            steps: 1,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    ),],
);
