//! Soul Sculptor — {2}{W} — Creature — Human
//! Oracle: {1}{W}, {T}: Target creature becomes an enchantment and loses all abilities until a player casts a creature spell.
//! Set: USG #53 — Urza's Saga | Scryfall ID: fe6ef073-1c83-47a4-b19d-86fb8bed5db9 | Oracle ID: d407abd0-69a9-405d-ab69-75b379a45780
// PARTIAL — duration until a player casts a creature spell is not supported; retains only its creature body

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SOUL_SCULPTOR,
    oracle_id = "d407abd0-69a9-405d-ab69-75b379a45780",
    scryfall_id = "fe6ef073-1c83-47a4-b19d-86fb8bed5db9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "duration until a player casts a creature spell is not supported; retains only its creature body",
    ),
    faces = &[face!(
        name = "Soul Sculptor",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
    // NOT SUPPORTED: duration until a player casts a creature spell is not supported; retains only its creature body.
    abilities = &[],
);
