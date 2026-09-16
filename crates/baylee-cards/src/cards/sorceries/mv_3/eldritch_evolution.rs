//! Eldritch Evolution — {1}{G}{G} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Search your library for a creature card with mana value X or less, where X is 2 plus the sacrificed creature's mana value. Put that card onto the battlefield, then shuffle. Exile Eldritch Evolution.
//! Set: INR #195 — Innistrad Remastered | Scryfall ID: 606caf13-c0d3-4a61-9a1a-32f13b6448ab | Oracle ID: 0f77c0c9-4dc4-489a-b547-e93287c4d1a5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ELDRITCH_EVOLUTION,
    oracle_id = "0f77c0c9-4dc4-489a-b547-e93287c4d1a5",
    scryfall_id = "606caf13-c0d3-4a61-9a1a-32f13b6448ab",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Eldritch Evolution",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
