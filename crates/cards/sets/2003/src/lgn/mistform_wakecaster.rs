//! Mistform Wakecaster — {4}{U} — Creature — Illusion
//! Oracle: Flying
//! Oracle: {1}: This creature becomes the creature type of your choice until end of turn.
//! Oracle: {2}{U}{U}, {T}: Choose a creature type. Each creature you control becomes that type until end of turn.
//! Set: LGN #48 — Legions | Scryfall ID: 1e5cbfb9-9bd0-4f8b-a444-a480de4b9662 | Oracle ID: 7108f2a2-1541-4bbf-aeb8-2a892a6dc337
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTFORM_WAKECASTER,
    oracle_id = "7108f2a2-1541-4bbf-aeb8-2a892a6dc337",
    scryfall_id = "1e5cbfb9-9bd0-4f8b-a444-a480de4b9662",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mistform Wakecaster",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ILLUSION],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
