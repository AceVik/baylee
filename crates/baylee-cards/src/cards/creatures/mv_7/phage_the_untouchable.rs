//! Phage the Untouchable — {3}{B}{B}{B}{B} — Legendary Creature — Avatar Minion
//! Oracle: When Phage enters, if you didn't cast it from your hand, you lose the game.
//! Oracle: Whenever Phage deals combat damage to a creature, destroy that creature. It can't be regenerated.
//! Oracle: Whenever Phage deals combat damage to a player, that player loses the game.
//! Set: CNS #120 — Conspiracy | Scryfall ID: d497a5a3-65fb-4c12-b3f2-8ce4cf4e0f6f | Oracle ID: 09d01406-b529-4c3b-900e-d8c889a4c400
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHAGE_THE_UNTOUCHABLE,
    oracle_id = "09d01406-b529-4c3b-900e-d8c889a4c400",
    scryfall_id = "d497a5a3-65fb-4c12-b3f2-8ce4cf4e0f6f",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Phage the Untouchable",
        mana_cost = mana!("{3}{B}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::AVATAR, subtypes::creature::MINION],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
