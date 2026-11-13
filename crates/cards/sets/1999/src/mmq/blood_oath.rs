//! Blood Oath — {3}{R} — Instant
//! Oracle: Choose a card type. Target opponent reveals their hand. Blood Oath deals 3 damage to that player for each card of the chosen type revealed this way. (Artifact, battle, creature, enchantment, instant, kindred, land, planeswalker, and sorcery are card types.)
//! Set: MMQ #177 — Mercadian Masques | Scryfall ID: f1556a12-ff45-4a12-988a-63615b3799a9 | Oracle ID: 877c34ca-6eff-42ce-b4d7-8a2071d0290a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLOOD_OATH,
    oracle_id = "877c34ca-6eff-42ce-b4d7-8a2071d0290a",
    scryfall_id = "f1556a12-ff45-4a12-988a-63615b3799a9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Blood Oath",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
