//! Snowfall — {2}{U} — Enchantment
//! Oracle: Cumulative upkeep {U} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Whenever an Island is tapped for mana, its controller may add an additional {U}. If that Island is snow, its controller may add an additional {U}{U} instead. Spend this mana only to pay cumulative upkeep costs.
//! Set: ICE #101 — Ice Age | Scryfall ID: 788ed793-3993-4a63-b9f9-9ac3947c3108 | Oracle ID: 5ffed544-1656-4753-ac8f-ad8fb4442c7f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SNOWFALL,
    oracle_id = "5ffed544-1656-4753-ac8f-ad8fb4442c7f",
    scryfall_id = "788ed793-3993-4a63-b9f9-9ac3947c3108",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Snowfall",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
