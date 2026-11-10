//! Hermit Druid — {1}{G} — Creature — Human Druid
//! Oracle: {G}, {T}: Reveal cards from the top of your library until you reveal a basic land card. Put that card into your hand and all other cards revealed this way into your graveyard.
//! Set: INR #202 — Innistrad Remastered | Scryfall ID: 39c66895-9c2d-49db-8261-e300a69b6cd5 | Oracle ID: 16f6438d-2a29-41cb-bf0c-4d02bd66112b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HERMIT_DRUID,
    oracle_id = "16f6438d-2a29-41cb-bf0c-4d02bd66112b",
    scryfall_id = "39c66895-9c2d-49db-8261-e300a69b6cd5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Hermit Druid",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::DRUID],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
