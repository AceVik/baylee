//! Iridescent Drake — {3}{U} — Creature — Drake
//! Oracle: Flying
//! Oracle: When this creature enters, put target Aura card from a graveyard onto the battlefield under your control attached to this creature.
//! Set: UMA #59 — Ultimate Masters | Scryfall ID: 5c99c962-aa33-4e19-96b2-7a05b97d06d4 | Oracle ID: f8cf31db-6c33-4bfb-a24c-f22644ccd377
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::IRIDESCENT_DRAKE,
    oracle_id = "f8cf31db-6c33-4bfb-a24c-f22644ccd377",
    scryfall_id = "5c99c962-aa33-4e19-96b2-7a05b97d06d4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Iridescent Drake",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAKE],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
