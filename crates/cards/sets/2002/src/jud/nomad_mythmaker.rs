//! Nomad Mythmaker — {2}{W} — Creature — Human Nomad Cleric
//! Oracle: {W}, {T}: Put target Aura card from a graveyard onto the battlefield under your control attached to a creature you control.
//! Set: 10E #30 — Tenth Edition | Scryfall ID: 0468f109-8020-4a74-8ffd-bba771362f76 | Oracle ID: 591ce7dc-8407-43ed-acfc-e3d8fd547125
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NOMAD_MYTHMAKER,
    oracle_id = "591ce7dc-8407-43ed-acfc-e3d8fd547125",
    scryfall_id = "0468f109-8020-4a74-8ffd-bba771362f76",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Nomad Mythmaker",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::NOMAD,
            subtypes::creature::CLERIC
        ],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
