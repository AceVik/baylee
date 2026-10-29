//! Goblin Grappler — {R} — Creature — Goblin
//! Oracle: Provoke (Whenever this creature attacks, you may have target creature defending player controls untap and block it if able.)
//! Set: LGN #100 — Legions | Scryfall ID: 5c948872-295c-41b9-8094-db7db7578b0d | Oracle ID: 48c1c84c-f690-42ce-9c5a-dd09b1e4197c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_GRAPPLER,
    oracle_id = "48c1c84c-f690-42ce-9c5a-dd09b1e4197c",
    scryfall_id = "5c948872-295c-41b9-8094-db7db7578b0d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin Grappler",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
