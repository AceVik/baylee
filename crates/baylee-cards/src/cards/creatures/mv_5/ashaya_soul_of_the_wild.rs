//! Ashaya, Soul of the Wild — {3}{G}{G} — Legendary Creature — Elemental
//! Oracle: Ashaya's power and toughness are each equal to the number of lands you control.
//! Oracle: Nontoken creatures you control are Forest lands in addition to their other types. (They're still affected by summoning sickness.)
//! Set: DSC #170 — Duskmourn: House of Horror Commander | Scryfall ID: 0a74b4e6-f6c9-4fef-a83c-a285a541e720 | Oracle ID: 162572f2-1757-42e9-bd97-e6bd9a762c0e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ASHAYA_SOUL_OF_THE_WILD,
    oracle_id = "162572f2-1757-42e9-bd97-e6bd9a762c0e",
    scryfall_id = "0a74b4e6-f6c9-4fef-a83c-a285a541e720",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Ashaya, Soul of the Wild",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
