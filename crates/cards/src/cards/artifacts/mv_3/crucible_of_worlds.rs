//! Crucible of Worlds — {3} — Artifact
//! Oracle: You may play lands from your graveyard.
//! Set: 2X2 #303 — Double Masters 2022 | Scryfall ID: 7f4893ef-f983-418b-b7a4-5f073c844545 | Oracle ID: 33c722cf-b4bf-431f-aefd-ee96241a7fbf
// IMPLEMENTED — the whole card is one permission, stated as a static
// ability with `Modifier::PlayLandsFromGraveyard` over `Filter::Any` (a
// player-scoped modifier, not one about an object). The extra land drop is
// a different sentence and is not printed here, so `ExtraLandDrops` is not
// written.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRUCIBLE_OF_WORLDS,
    oracle_id = "33c722cf-b4bf-431f-aefd-ee96241a7fbf",
    scryfall_id = "7f4893ef-f983-418b-b7a4-5f073c844545",
    faces = &[face!(
        name = "Crucible of Worlds",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::PlayLandsFromGraveyard
    )],
);
