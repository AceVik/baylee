//! Unlicensed Hearse — {2} — Artifact — Vehicle
//! Oracle: {T}: Exile up to two target cards from a single graveyard.
//! Oracle: Unlicensed Hearse's power and toughness are each equal to the number of cards exiled with it.
//! Oracle: Crew 2
//! Set: SNC #246 — Streets of New Capenna | Scryfall ID: 93ee60f7-31dd-4bc6-b71f-57a1a0d19d20 | Oracle ID: c640654c-487e-4a2c-aced-126ed835b78f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UNLICENSED_HEARSE,
    oracle_id = "c640654c-487e-4a2c-aced-126ed835b78f",
    scryfall_id = "93ee60f7-31dd-4bc6-b71f-57a1a0d19d20",
    faces = &[face!(
        name = "Unlicensed Hearse",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::VEHICLE],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
