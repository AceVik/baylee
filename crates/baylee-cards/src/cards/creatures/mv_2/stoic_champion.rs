//! Stoic Champion — {W}{W} — Creature — Human Soldier
//! Oracle: Whenever a player cycles a card, this creature gets +2/+2 until end of turn.
//! Set: VMA #50 — Vintage Masters | Scryfall ID: cd693780-85df-4cea-be03-43c1d80ab740 | Oracle ID: 1203a80f-11ac-46ed-aa2a-5a6782b6303d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STOIC_CHAMPION,
    oracle_id = "1203a80f-11ac-46ed-aa2a-5a6782b6303d",
    scryfall_id = "cd693780-85df-4cea-be03-43c1d80ab740",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Stoic Champion",
        mana_cost = mana!("{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
