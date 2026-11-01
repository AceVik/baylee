//! Infectious Rage — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/-1.
//! Oracle: When enchanted creature dies, choose a creature at random this Aura can enchant. Return this card to the battlefield attached to that creature.
//! Set: JUD #92 — Judgment | Scryfall ID: 8569cdf7-e0e9-4733-98b5-56fac216fad3 | Oracle ID: ed0402d7-02cb-4763-a661-82280d6c9af8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::INFECTIOUS_RAGE,
    oracle_id = "ed0402d7-02cb-4763-a661-82280d6c9af8",
    scryfall_id = "8569cdf7-e0e9-4733-98b5-56fac216fad3",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Infectious Rage",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
