//! Wormfang Drake — {2}{U} — Creature — Nightmare Drake
//! Oracle: Flying
//! Oracle: When this creature enters, sacrifice it unless you exile a creature you control other than this creature.
//! Oracle: When this creature leaves the battlefield, return the exiled card to the battlefield under its owner's control.
//! Set: DMR #74 — Dominaria Remastered | Scryfall ID: 501128c7-c541-4de8-a041-9367c032dbb3 | Oracle ID: d37c4e77-5023-4d13-9cfe-a43f513364bf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WORMFANG_DRAKE,
    oracle_id = "d37c4e77-5023-4d13-9cfe-a43f513364bf",
    scryfall_id = "501128c7-c541-4de8-a041-9367c032dbb3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Wormfang Drake",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::DRAKE],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
