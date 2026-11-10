//! Haunted Cadaver — {3}{B} — Creature — Zombie
//! Oracle: Whenever this creature deals combat damage to a player, you may sacrifice it. If you do, that player discards three cards.
//! Oracle: Morph {1}{B} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #154 — Onslaught | Scryfall ID: a164420c-3619-4f5e-81cf-2aa5a4553bc3 | Oracle ID: 7f46839f-175d-4292-8057-c3e0da206075
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HAUNTED_CADAVER,
    oracle_id = "7f46839f-175d-4292-8057-c3e0da206075",
    scryfall_id = "a164420c-3619-4f5e-81cf-2aa5a4553bc3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Haunted Cadaver",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
