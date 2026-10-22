//! Squandered Resources — {B}{G} — Enchantment
//! Oracle: Sacrifice a land: Add one mana of any type the sacrificed land could produce.
//! Set: VIS #137 — Visions | Scryfall ID: fcddbea7-3025-47b1-a597-2d2b2711fb81 | Oracle ID: 620d0572-6d22-43e2-83a9-000e605c8a2b
// PARTIAL: sacrificing a land produces one mana of any color; restricting the produced mana to the types the sacrificed land could produce (including colorless) is not yet modeled in ManaSource.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SQUANDERED_RESOURCES,
    oracle_id = "620d0572-6d22-43e2-83a9-000e605c8a2b",
    scryfall_id = "fcddbea7-3025-47b1-a597-2d2b2711fb81",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    coverage = Coverage::Partial(
        "sacrificing a land produces one mana of any color; restricting the produced mana to the types the sacrificed land could produce (including colorless) is not yet modeled in ManaSource",
    ),
    faces = &[face!(
        name = "Squandered Resources",
        mana_cost = mana!("{B}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[mana_ability!(
        cost!(Sacrifice(&Filter::LAND)),
        &[Effect::mana_of_any_color()]
    )],
);
