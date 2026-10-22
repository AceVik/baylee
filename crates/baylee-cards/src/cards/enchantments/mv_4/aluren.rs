//! Aluren — {2}{G}{G} — Enchantment
//! Oracle: Any player may cast creature spells with mana value 3 or less without paying their mana costs and as though they had flash.
//! Set: TPR #165 — Tempest Remastered | Scryfall ID: 03b69c39-52d8-4ee6-a3f5-60899c1fe6e6 | Oracle ID: 44e02e70-07ab-4795-8037-55185bd11d54
// PARTIAL — Reduces the generic mana cost of creature spells with mana value 3 or less by {3} and lets you cast creature spells as though they had flash; casting creature spells with mana value 3 or less without paying their colored mana costs and extending the flash permission to other players are not yet modeled.

use baylee_cards_dsl::prelude::*;

const CREATURE_MV_3_OR_LESS: Filter = Filter::And(&[Filter::CREATURE, Filter::CmcAtMost(3)]);

card!(
    index = index::ALUREN,
    oracle_id = "44e02e70-07ab-4795-8037-55185bd11d54",
    scryfall_id = "03b69c39-52d8-4ee6-a3f5-60899c1fe6e6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Reduces the generic mana cost of creature spells with mana value 3 or less by {3} and lets you cast creature spells as though they had flash; casting creature spells with mana value 3 or less without paying their colored mana costs and extending the flash permission to other players are not yet modeled."
    ),
    faces = &[face!(
        name = "Aluren",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        static_ability!(CREATURE_MV_3_OR_LESS, Modifier::SpellsCostLess(3)),
        static_ability!(
            Filter::This,
            Modifier::SpellsHaveFlash(Some(TypeSet::CREATURE))
        ),
    ],
);
