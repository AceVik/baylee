//! Class-F Shuttlecraft — {3} — Artifact — Spacecraft
//! Oracle: {T}: Add one mana of any color.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 6+.)
//! Oracle: 6+ | Flying, vigilance
//! Set: TRK #259 — Star Trek | Scryfall ID: 55e0a183-4145-472f-8bed-a1f247febe06 | Oracle ID: 4ccc91f2-4d6e-4d64-b438-3d8b4361040a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CLASS_F_SHUTTLECRAFT,
    oracle_id = "4ccc91f2-4d6e-4d64-b438-3d8b4361040a",
    scryfall_id = "55e0a183-4145-472f-8bed-a1f247febe06",
    faces = &[face!(
        name = "Class-F Shuttlecraft",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
