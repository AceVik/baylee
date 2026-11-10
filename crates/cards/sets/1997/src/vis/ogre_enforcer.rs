//! Ogre Enforcer — {3}{R}{R} — Creature — Ogre
//! Oracle: This creature can't be destroyed by lethal damage unless lethal damage dealt by a single source is marked on it.
//! Set: VIS #89 — Visions | Scryfall ID: b0f072d6-7489-4eb0-8c53-1fa42ad806a4 | Oracle ID: 86cc83d8-4a70-4b97-aa4d-160afd4cd6cb
// PARTIAL: 4/4 Ogre; requiring lethal damage from a single source to destroy it by damage is not yet supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: This creature can't be destroyed by lethal damage unless lethal damage dealt by a single source is marked on it.
card!(
    index = index::OGRE_ENFORCER,
    oracle_id = "86cc83d8-4a70-4b97-aa4d-160afd4cd6cb",
    scryfall_id = "b0f072d6-7489-4eb0-8c53-1fa42ad806a4",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "4/4 Ogre; requiring lethal damage from a single source to destroy it by damage is not yet supported"
    ),
    faces = &[face!(
        name = "Ogre Enforcer",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OGRE],
        power = Some(4),
        toughness = Some(4),
    ),],
);
