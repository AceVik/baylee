//! Harvester Druid — {1}{G} — Creature — Human Druid
//! Oracle: {T}: Add one mana of any color that a land you control could produce.
//! Set: JUD #120 — Judgment | Scryfall ID: 97337e6e-1b3f-43a2-91f2-ca8f6c5dea88 | Oracle ID: c2b4f43a-6609-43ca-ad0d-897b6f8cb22f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HARVESTER_DRUID,
    oracle_id = "c2b4f43a-6609-43ca-ad0d-897b6f8cb22f",
    scryfall_id = "97337e6e-1b3f-43a2-91f2-ca8f6c5dea88",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Harvester Druid",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
