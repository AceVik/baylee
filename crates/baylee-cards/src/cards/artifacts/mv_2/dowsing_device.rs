//! Dowsing Device // Geode Grotto — {1}{R} — Artifact // Land — Cave
//! Oracle: Whenever this artifact or another artifact you control enters, up to one target creature you control gets +1/+0 and gains haste until end of turn. Then transform this artifact if you control four or more artifacts.
//! Oracle: (Transforms from Dowsing Device.)
//! Oracle: {T}: Add {R}.
//! Oracle: {2}{R}, {T}: Until end of turn, target creature gains haste and gets +X/+0, where X is the number of artifacts you control. Activate only as a sorcery.
//! Set: LCI #146 — The Lost Caverns of Ixalan | Scryfall ID: 3d715e9f-223d-462e-8ce3-eebbaf1cd021 | Oracle ID: 2f4374f6-c695-4a5d-a6d6-0e41eaa587ca
//! Face: Dowsing Device — {1}{R} — Artifact
//! Face: Geode Grotto —  — Land — Cave
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DOWSING_DEVICE,
    oracle_id = "2f4374f6-c695-4a5d-a6d6-0e41eaa587ca",
    scryfall_id = "3d715e9f-223d-462e-8ce3-eebbaf1cd021",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Dowsing Device",
            mana_cost = mana!("{1}{R}"),
            types = TypeSet::ARTIFACT,
        ),
        face!(
            name = "Geode Grotto",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::CAVE],
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
