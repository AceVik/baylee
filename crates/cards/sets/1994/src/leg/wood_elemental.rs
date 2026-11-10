//! Wood Elemental — {3}{G} — Creature — Elemental
//! Oracle: As this creature enters, sacrifice any number of untapped Forests.
//! Oracle: Wood Elemental's power and toughness are each equal to the number of Forests sacrificed as it entered.
//! Set: ME4 #175 — Masters Edition IV | Scryfall ID: 71ced69c-921c-4a31-a213-0faf927134ef | Oracle ID: 2b6da458-e075-4696-a74f-c8846e0cc370

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WOOD_ELEMENTAL,
    oracle_id = "2b6da458-e075-4696-a74f-c8846e0cc370",
    scryfall_id = "71ced69c-921c-4a31-a213-0faf927134ef",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Wood Elemental",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        enter_modifiers = &[EnterModifier::SacrificeForPowerToughness(&Filter::And(&[
            Filter::ControlledByYou,
            Filter::Untapped,
            Filter::HasSubtype(subtypes::land::FOREST)
        ]))],
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(0),
        toughness = Some(0),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CharacteristicPT {
            count: PtCount::EntrySacrifices,
            toughness_plus: 0,
        }
    )],
);
