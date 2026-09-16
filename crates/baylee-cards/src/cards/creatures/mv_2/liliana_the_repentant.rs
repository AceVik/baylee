//! Liliana the Repentant — {1}{B} — Legendary Creature — Human Warlock
//! Oracle: Whenever another creature or planeswalker you control enters, mill two cards.
//! Oracle: Exhaust — {5}{B}: Return target creature or planeswalker card from your graveyard to the battlefield. Put a +1/+1 counter on Liliana. Activate only as a sorcery. (Activate each exhaust ability only once.)
//! Set: FRA #231 — Reality Fracture | Scryfall ID: 1eb25a6c-d6b4-465d-990e-f1ab86b26b69 | Oracle ID: 5eb4403f-f199-4f75-a7c6-e76783f9b07d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LILIANA_THE_REPENTANT,
    oracle_id = "5eb4403f-f199-4f75-a7c6-e76783f9b07d",
    scryfall_id = "1eb25a6c-d6b4-465d-990e-f1ab86b26b69",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Liliana the Repentant",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARLOCK],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
