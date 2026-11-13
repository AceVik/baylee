//! Diligent Farmhand — {G} — Creature — Human Druid
//! Oracle: {1}{G}, Sacrifice this creature: Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.
//! Oracle: If this card is in a graveyard, effects from spells named Muscle Burst count it as a card named Muscle Burst.
//! Set: ODY #237 — Odyssey | Scryfall ID: 7bb40e09-6855-46d5-9bc9-bc6b2b0d7653 | Oracle ID: 5f6aa548-0ab3-451f-b291-10cac3f3ea1a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DILIGENT_FARMHAND,
    oracle_id = "5f6aa548-0ab3-451f-b291-10cac3f3ea1a",
    scryfall_id = "7bb40e09-6855-46d5-9bc9-bc6b2b0d7653",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Diligent Farmhand",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
