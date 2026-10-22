//! Desolation — {1}{B}{B} — Enchantment
//! Oracle: At the beginning of each end step, each player who tapped a land for mana this turn sacrifices a land of their choice. This enchantment deals 2 damage to each player who sacrificed a Plains this way.
//! Set: VIS #58 — Visions | Scryfall ID: 3b186460-d2af-4912-ba19-95b2cb5f1639 | Oracle ID: a80c3af8-470e-4e95-9579-cfdf18fbdbfa
// PARTIAL — resolves as an enchantment; no end-step trigger checking which players tapped a land for mana this turn to make them sacrifice a land and deal 2 damage if a Plains was sacrificed exists yet.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DESOLATION,
    oracle_id = "a80c3af8-470e-4e95-9579-cfdf18fbdbfa",
    scryfall_id = "3b186460-d2af-4912-ba19-95b2cb5f1639",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "resolves as an enchantment; no end-step trigger checking which players tapped a land for mana this turn to make them sacrifice a land and deal 2 damage if a Plains was sacrificed exists yet",
    ),
    faces = &[face!(
        name = "Desolation",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);
