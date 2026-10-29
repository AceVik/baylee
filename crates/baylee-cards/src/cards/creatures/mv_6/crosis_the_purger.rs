//! Crosis, the Purger — {3}{U}{B}{R} — Legendary Creature — Dragon
//! Oracle: Flying
//! Oracle: Whenever Crosis deals combat damage to a player, you may pay {2}{B}. If you do, choose a color, then that player reveals their hand and discards all cards of that color.
//! Set: C17 #168 — Commander 2017 | Scryfall ID: be667162-cd28-403c-b65f-dadc5459e757 | Oracle ID: f1ae1dcd-76cd-4714-9df2-1ab0ebd2ca41
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CROSIS_THE_PURGER,
    oracle_id = "f1ae1dcd-76cd-4714-9df2-1ab0ebd2ca41",
    scryfall_id = "be667162-cd28-403c-b65f-dadc5459e757",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Crosis, the Purger",
        mana_cost = mana!("{3}{U}{B}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
