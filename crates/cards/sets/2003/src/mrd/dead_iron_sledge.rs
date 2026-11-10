//! Dead-Iron Sledge — {1} — Artifact — Equipment
//! Oracle: Whenever equipped creature blocks or becomes blocked by a creature, destroy both creatures.
//! Oracle: Equip {2} ({2}: Attach to target creature you control. Equip only as a sorcery.)
//! Set: MRD #162 — Mirrodin | Scryfall ID: 1dc312c0-14da-4b23-8293-6fa41bdd3167 | Oracle ID: e1ee79de-e4c3-48f9-87a7-18f72e2f6116
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BOTH_CREATURES: Filter = Filter::Or(&[Filter::AttachedToBySource, Filter::Reference]);

card!(
    index = index::DEAD_IRON_SLEDGE,
    oracle_id = "e1ee79de-e4c3-48f9-87a7-18f72e2f6116",
    scryfall_id = "1dc312c0-14da-4b23-8293-6fa41bdd3167",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dead-Iron Sledge",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
    abilities = &[
        equip!(cost!("{2}")),
        triggered!(
            Trigger::AttachedBlocksOrBecomesBlockedBy(&Filter::CREATURE),
            &[Effect::destroy_all(&BOTH_CREATURES)],
        ),
    ],
);
