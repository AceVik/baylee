//! Ninja's Kunai — {1} — Artifact — Equipment
//! Oracle: Equipped creature has "{1}, {T}, Sacrifice Ninja's Kunai: Ninja's Kunai deals 3 damage to any target."
//! Oracle: Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
//! Set: NEO #252 — Kamigawa: Neon Dynasty | Scryfall ID: af6ae567-c77a-416a-b609-4df3d0dd2e18 | Oracle ID: fa5aefaf-432f-41d0-8e54-0e8b7bc7ba98
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NINJA_S_KUNAI,
    oracle_id = "fa5aefaf-432f-41d0-8e54-0e8b7bc7ba98",
    scryfall_id = "af6ae567-c77a-416a-b609-4df3d0dd2e18",
    faces = &[face!(
        name = "Ninja's Kunai",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
