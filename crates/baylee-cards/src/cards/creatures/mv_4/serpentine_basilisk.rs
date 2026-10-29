//! Serpentine Basilisk — {2}{G}{G} — Creature — Basilisk
//! Oracle: Whenever this creature deals combat damage to a creature, destroy that creature at end of combat.
//! Oracle: Morph {1}{G}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #280 — Onslaught | Scryfall ID: 4052a5af-20b2-4817-8c94-78d488ee220f | Oracle ID: bf0f3d2a-6b6a-4822-888d-1d897a8b5aea
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SERPENTINE_BASILISK,
    oracle_id = "bf0f3d2a-6b6a-4822-888d-1d897a8b5aea",
    scryfall_id = "4052a5af-20b2-4817-8c94-78d488ee220f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Serpentine Basilisk",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BASILISK],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
