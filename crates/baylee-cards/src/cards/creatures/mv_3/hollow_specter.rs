//! Hollow Specter — {1}{B}{B} — Creature — Specter
//! Oracle: Flying
//! Oracle: Whenever this creature deals combat damage to a player, you may pay {X}. If you do, that player reveals X cards from their hand and you choose one of them. That player discards that card.
//! Set: LGN #75 — Legions | Scryfall ID: 2db779fd-0e01-417b-aee2-786db2c0b8c8 | Oracle ID: 797aed94-c409-4441-88c1-baba7e212696
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HOLLOW_SPECTER,
    oracle_id = "797aed94-c409-4441-88c1-baba7e212696",
    scryfall_id = "2db779fd-0e01-417b-aee2-786db2c0b8c8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Hollow Specter",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPECTER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
