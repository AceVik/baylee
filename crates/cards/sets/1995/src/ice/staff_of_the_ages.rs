//! Staff of the Ages — {3} — Artifact
//! Oracle: Creatures with landwalk abilities can be blocked as though they didn't have those abilities.
//! Set: ICE #340 — Ice Age | Scryfall ID: 5c709836-55b6-4de9-b190-b5f66dc53c87 | Oracle ID: 86af21ba-58ad-45e8-a316-5411c870e415

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STAFF_OF_THE_AGES,
    oracle_id = "86af21ba-58ad-45e8-a316-5411c870e415",
    scryfall_id = "5c709836-55b6-4de9-b190-b5f66dc53c87",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Staff of the Ages",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::CREATURE,
        Modifier::BlockableWithoutAnyLandwalk,
    )],
);
