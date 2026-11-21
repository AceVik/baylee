//! Shocker, Unshakable — {4}{R}{R} — Legendary Creature — Human Rogue Villain
//! Oracle: During your turn, Shocker has first strike.
//! Oracle: Vibro-Shock Gauntlets — When Shocker enters, he deals 2 damage to target creature and 2 damage to that creature's controller.
//! Set: SPM #89 — Marvel's Spider-Man | Scryfall ID: 8b2c0d9a-364a-4823-aa4c-fe473d4463f0 | Oracle ID: 564dac59-97f7-4988-9f30-d82a891e29ef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHOCKER_UNSHAKABLE,
    oracle_id = "564dac59-97f7-4988-9f30-d82a891e29ef",
    scryfall_id = "8b2c0d9a-364a-4823-aa4c-fe473d4463f0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Shocker, Unshakable",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::ROGUE,
            subtypes::creature::VILLAIN
        ],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
