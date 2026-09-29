//! Regeneration — {1}{G} — Enchantment — Aura
//! Oracle: Enchant creature (Target a creature as you cast this. This card enters attached to that creature.)
//! Oracle: {G}: Regenerate enchanted creature. (The next time that creature would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
//! Set: 10E #290 — Tenth Edition | Scryfall ID: 4b848c69-0f64-4631-84b9-6597c359f3ba | Oracle ID: 89390a33-b289-4edd-a114-6616e49a49c2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::REGENERATION,
    oracle_id = "89390a33-b289-4edd-a114-6616e49a49c2",
    scryfall_id = "4b848c69-0f64-4631-84b9-6597c359f3ba",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Regeneration",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
