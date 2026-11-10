//! Hammer Mage — {1}{R} — Creature — Human Spellshaper
//! Oracle: {X}{R}, {T}, Discard a card: Destroy all artifacts with mana value X or less.
//! Set: MMQ #193 — Mercadian Masques | Scryfall ID: b959d7ad-a78e-439f-9225-4dbb89f490d7 | Oracle ID: f642a43c-f8cc-44ee-ac6a-5d4b3fbc49af
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HAMMER_MAGE,
    oracle_id = "f642a43c-f8cc-44ee-ac6a-5d4b3fbc49af",
    scryfall_id = "b959d7ad-a78e-439f-9225-4dbb89f490d7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Hammer Mage",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SPELLSHAPER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
