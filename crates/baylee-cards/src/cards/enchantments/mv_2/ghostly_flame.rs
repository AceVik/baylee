//! Ghostly Flame — {B}{R} — Enchantment
//! Oracle: Black and/or red permanents and spells are colorless sources of damage.
//! Set: ICE #292 — Ice Age | Scryfall ID: 6314344b-6493-4142-9c76-da9b90b8d3e1 | Oracle ID: 6a461ca3-7923-4df4-9f15-4072545da117
use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "Black and/or red permanents and spells are colorless sources of damage."
card!(
    index = index::GHOSTLY_FLAME,
    oracle_id = "6a461ca3-7923-4df4-9f15-4072545da117",
    scryfall_id = "6314344b-6493-4142-9c76-da9b90b8d3e1",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Partial(
        "treating colored permanents and spells as colorless damage sources not supported"
    ),
    faces = &[face!(
        name = "Ghostly Flame",
        mana_cost = mana!("{B}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);
