//! World Shaper — {3}{G} — Creature — Merfolk Shaman
//! Oracle: Whenever this creature attacks, you may mill three cards.
//! Oracle: When this creature dies, return all land cards from your graveyard to the battlefield tapped.
//! Set: OTC #214 — Outlaws of Thunder Junction Commander | Scryfall ID: cc765da4-4bca-4250-80e4-05575d6fa98c | Oracle ID: 3c075bb6-1831-4521-bd8d-4ed2825ae796
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WORLD_SHAPER,
    oracle_id = "3c075bb6-1831-4521-bd8d-4ed2825ae796",
    scryfall_id = "cc765da4-4bca-4250-80e4-05575d6fa98c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "World Shaper",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::SHAMAN],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
