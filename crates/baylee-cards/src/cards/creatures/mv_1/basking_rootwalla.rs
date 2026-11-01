//! Basking Rootwalla — {G} — Creature — Lizard
//! Oracle: {1}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
//! Oracle: Madness {0} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)
//! Set: UMA #156 — Ultimate Masters | Scryfall ID: 26bfde99-7761-48e1-851a-522f888d0f6c | Oracle ID: b2ec080f-ad5d-4274-91e3-bc6ac89fc3e4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BASKING_ROOTWALLA,
    oracle_id = "b2ec080f-ad5d-4274-91e3-bc6ac89fc3e4",
    scryfall_id = "26bfde99-7761-48e1-851a-522f888d0f6c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Basking Rootwalla",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
