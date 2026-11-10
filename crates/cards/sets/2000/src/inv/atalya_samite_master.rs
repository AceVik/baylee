//! Atalya, Samite Master — {3}{W}{W} — Legendary Creature — Human Cleric
//! Oracle: {X}, {T}: Choose one —
//! Oracle: • Prevent the next X damage that would be dealt to target creature this turn. Spend only white mana on X.
//! Oracle: • You gain X life. Spend only white mana on X.
//! Set: INV #4 — Invasion | Scryfall ID: 90500e7a-f76d-453a-bda0-d56d3f7c7534 | Oracle ID: 84587724-af96-4396-a0b3-1133d492d915
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ATALYA_SAMITE_MASTER,
    oracle_id = "84587724-af96-4396-a0b3-1133d492d915",
    scryfall_id = "90500e7a-f76d-453a-bda0-d56d3f7c7534",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Atalya, Samite Master",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
