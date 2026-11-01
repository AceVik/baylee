//! Dromar, the Banisher — {3}{W}{U}{B} — Legendary Creature — Dragon
//! Oracle: Flying
//! Oracle: Whenever Dromar deals combat damage to a player, you may pay {2}{U}. If you do, choose a color, then return all creatures of that color to their owners' hands.
//! Set: INV #244 — Invasion | Scryfall ID: cfcc3c72-fff5-454c-814c-eb952fd23ba9 | Oracle ID: 35ad7f82-a1b1-4aa7-a8b0-3f99ca2cff52
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DROMAR_THE_BANISHER,
    oracle_id = "35ad7f82-a1b1-4aa7-a8b0-3f99ca2cff52",
    scryfall_id = "cfcc3c72-fff5-454c-814c-eb952fd23ba9",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Dromar, the Banisher",
        mana_cost = mana!("{3}{W}{U}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
