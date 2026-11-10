//! Shieldmage Elder — {5}{W} — Creature — Human Cleric Wizard
//! Oracle: Tap two untapped Clerics you control: Prevent all damage target creature would deal this turn.
//! Oracle: Tap two untapped Wizards you control: Prevent all damage target spell would deal this turn.
//! Set: ONS #54 — Onslaught | Scryfall ID: efa2d660-7c93-4087-a6e5-49c2ad21eb5a | Oracle ID: 943bbe5b-76cd-478d-a1fe-d4e557df3469
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHIELDMAGE_ELDER,
    oracle_id = "943bbe5b-76cd-478d-a1fe-d4e557df3469",
    scryfall_id = "efa2d660-7c93-4087-a6e5-49c2ad21eb5a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Shieldmage Elder",
        mana_cost = mana!("{5}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::CLERIC,
            subtypes::creature::WIZARD
        ],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
