//! Gerrard Capashen — {3}{W}{W} — Legendary Creature — Human Soldier
//! Oracle: At the beginning of your upkeep, you gain 1 life for each card in target opponent's hand.
//! Oracle: {3}{W}: Tap target creature. Activate only if Gerrard Capashen is attacking.
//! Set: DDE #46 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: 447c8c98-ac0e-4a12-a99e-f75d82b7a51f | Oracle ID: 27521e70-3a1f-4819-a111-c38216ffe7c3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GERRARD_CAPASHEN,
    oracle_id = "27521e70-3a1f-4819-a111-c38216ffe7c3",
    scryfall_id = "447c8c98-ac0e-4a12-a99e-f75d82b7a51f",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Gerrard Capashen",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
