//! Gratuitous Violence — {2}{R}{R}{R} — Enchantment
//! Oracle: If a creature you control would deal damage to a permanent or player, it deals double that damage instead.
//! Set: FDN #715 — Foundations | Scryfall ID: e89024c5-eef5-468c-92b4-e53dd212909c | Oracle ID: 7c340a39-4ee0-4ba1-bb66-6674f8020fda
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GRATUITOUS_VIOLENCE,
    oracle_id = "7c340a39-4ee0-4ba1-bb66-6674f8020fda",
    scryfall_id = "e89024c5-eef5-468c-92b4-e53dd212909c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Gratuitous Violence",
        mana_cost = mana!("{2}{R}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
