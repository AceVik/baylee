//! Nettling Imp — {2}{B} — Creature — Imp
//! Oracle: {T}: Choose target non-Wall creature the active player has controlled continuously since the beginning of the turn. That creature attacks this turn if able. Destroy it at the beginning of the next end step if it didn't attack this turn. Activate only during an opponent's turn, before attackers are declared.
//! Set: SUM #119 — Summer Magic / Edgar | Scryfall ID: 54039c4b-23c7-4e2c-8bd3-7a28714244b8 | Oracle ID: c58dfcbf-49e6-4ef0-bd31-ebd81b0cfa41
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NETTLING_IMP,
    oracle_id = "c58dfcbf-49e6-4ef0-bd31-ebd81b0cfa41",
    scryfall_id = "54039c4b-23c7-4e2c-8bd3-7a28714244b8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Nettling Imp",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::IMP],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
