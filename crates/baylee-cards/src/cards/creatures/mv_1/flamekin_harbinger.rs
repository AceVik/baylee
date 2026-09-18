//! Flamekin Harbinger — {R} — Creature — Elemental Shaman
//! Oracle: When this creature enters, you may search your library for an Elemental card, reveal it, then shuffle and put that card on top.
//! Set: HOP #53 — Planechase | Scryfall ID: 2796424c-f6c5-4851-87fb-145a58fe1f60 | Oracle ID: d6585e30-4ca0-4701-b274-b24f3508dd97
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLAMEKIN_HARBINGER,
    oracle_id = "d6585e30-4ca0-4701-b274-b24f3508dd97",
    scryfall_id = "2796424c-f6c5-4851-87fb-145a58fe1f60",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Flamekin Harbinger",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::SHAMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
