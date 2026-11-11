//! Spare Dagger — {1} — Artifact — Equipment
//! Oracle: Equipped creature gets +1/+0 and has "Whenever this creature attacks, you may sacrifice Spare Dagger. When you do, this creature deals 1 damage to any target."
//! Oracle: Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)
//! Set: AFR #250 — Adventures in the Forgotten Realms | Scryfall ID: ab6b0291-00f8-45bf-9f3a-ee43739c2b77 | Oracle ID: d895df97-e7e7-4f15-a79b-0ce8f1a252d3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPARE_DAGGER,
    oracle_id = "d895df97-e7e7-4f15-a79b-0ce8f1a252d3",
    scryfall_id = "ab6b0291-00f8-45bf-9f3a-ee43739c2b77",
    faces = &[face!(
        name = "Spare Dagger",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
