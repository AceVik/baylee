//! Legacy's Allure — {U}{U} — Enchantment
//! Oracle: At the beginning of your upkeep, you may put a treasure counter on this enchantment.
//! Oracle: Sacrifice this enchantment: Gain control of target creature with power less than or equal to the number of treasure counters on this enchantment. (This effect lasts indefinitely.)
//! Set: TPR #56 — Tempest Remastered | Scryfall ID: 2639c3c1-c631-4b2d-a311-a01bbea4753d | Oracle ID: 106ca7bd-6dad-43fe-af58-8667ebddb2eb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LEGACY_S_ALLURE,
    oracle_id = "106ca7bd-6dad-43fe-af58-8667ebddb2eb",
    scryfall_id = "2639c3c1-c631-4b2d-a311-a01bbea4753d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Legacy's Allure",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
