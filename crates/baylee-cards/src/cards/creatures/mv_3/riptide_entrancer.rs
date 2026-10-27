//! Riptide Entrancer — {1}{U}{U} — Creature — Human Wizard
//! Oracle: Whenever this creature deals combat damage to a player, you may sacrifice it. If you do, gain control of target creature that player controls. (This effect lasts indefinitely.)
//! Oracle: Morph {U}{U} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #108 — Onslaught | Scryfall ID: 2cd9abc9-f289-4294-bc0f-4addc8b92a4e | Oracle ID: d0bf59a2-347c-4c3e-85cf-b27a2af77192
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RIPTIDE_ENTRANCER,
    oracle_id = "d0bf59a2-347c-4c3e-85cf-b27a2af77192",
    scryfall_id = "2cd9abc9-f289-4294-bc0f-4addc8b92a4e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Riptide Entrancer",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
