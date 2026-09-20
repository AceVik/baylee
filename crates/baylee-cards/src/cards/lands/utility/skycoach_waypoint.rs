//! Skycoach Waypoint — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}: Target creature becomes prepared. (Only creatures with prepare spells can become prepared.)
//! Set: SOS #261 — Secrets of Strixhaven | Scryfall ID: 6747657b-5ce4-4dbd-b924-ca1f7119faf7 | Oracle ID: 2ac2b815-2d72-48e6-b43a-18884a74bf95
// PARTIAL — {T}: Add {C} is implemented; the prepare activation cannot be
// expressed, because the only prepared effect writes the marker onto the
// source.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SKYCOACH_WAYPOINT,
    oracle_id = "2ac2b815-2d72-48e6-b43a-18884a74bf95",
    scryfall_id = "6747657b-5ce4-4dbd-b924-ca1f7119faf7",
    faces = &[face!(name = "Skycoach Waypoint", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "{3}, {T}: Target creature becomes prepared — Effect::BecomePrepared puts the marker on the source and takes no target, and no variant gives it to another creature"
    ),
    // NOT SUPPORTED: {3}, {T}: Target creature becomes prepared. (Only creatures
    // with prepare spells can become prepared.)
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
